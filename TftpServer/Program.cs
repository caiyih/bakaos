#nullable enable
#pragma warning disable IDE1006

using System.Diagnostics;
using System.Net;
using System.Net.NetworkInformation;
using Tftp.Net;

namespace TftpServer
{
    class Program
    {
        private static string ServerDirectory = null!;
        private static readonly Dictionary<ITftpTransfer, TransferOperation> TransferOperations = new();

        private static volatile bool isRunning = true;

        static void Main(string[] args)
        {
            ServerDirectory = Environment.CurrentDirectory;

            if (args.Length >= 1)
            {
                string inputDirectory = args.First()!;

                if (Directory.Exists(inputDirectory))
                {
                    ServerDirectory = inputDirectory;
                }
                else
                {
                    Console.WriteLine($"Input directory '{inputDirectory}' not exist, using cwd.");
                }
            }

            ServerDirectory = Path.GetFullPath(ServerDirectory);

            Console.WriteLine($"Running TFTP server for directory: {ServerDirectory}");
            Console.WriteLine(string.Join(Environment.NewLine, GetLocalIPv4Addresses().Select(ip => $"Local IP: {ip}")));
            Console.WriteLine("Press Ctrl+C to close the server.");
            Console.WriteLine();

            Console.CancelKeyPress += (_, _) => isRunning = false;

            using (var server = new Tftp.Net.TftpServer())
            {
                server.OnReadRequest += new TftpServerEventHandler(server_OnReadRequest);
                server.OnWriteRequest += new TftpServerEventHandler(server_OnWriteRequest);
                server.Start();

                SpinWait.SpinUntil(() => { Thread.Sleep(100); return !isRunning; });
            }
        }

        static IEnumerable<string> GetLocalIPv4Addresses() =>
            NetworkInterface.GetAllNetworkInterfaces()
                .Where(ni => ni.NetworkInterfaceType != NetworkInterfaceType.Loopback && ni.OperationalStatus == OperationalStatus.Up)
                .SelectMany(ni => ni.GetIPProperties().UnicastAddresses)
                .Where(ua => ua.Address.AddressFamily == System.Net.Sockets.AddressFamily.InterNetwork)
                .Select(ua => ua.Address.ToString());

        static void server_OnWriteRequest(ITftpTransfer transfer, EndPoint client)
        {
            // Write operation is not allowed
            CancelTransfer(transfer, TftpErrorPacket.IllegalOperation);
        }

        static void server_OnReadRequest(ITftpTransfer transfer, EndPoint client)
        {
            string path = Path.Combine(ServerDirectory, transfer.Filename);
            FileInfo file = new FileInfo(path);

            //Is the file within the server directory?
            if (!file.FullName.StartsWith(ServerDirectory, StringComparison.InvariantCultureIgnoreCase))
            {
                CancelTransfer(transfer, TftpErrorPacket.AccessViolation);
            }
            else if (!file.Exists)
            {
                CancelTransfer(transfer, TftpErrorPacket.FileNotFound);
            }
            else
            {
                OutputTransferStatus(transfer, "Accepting request from " + client);
                StartTransfer(transfer, new FileStream(file.FullName, FileMode.Open, FileAccess.Read));
            }
        }

        private static void StartTransfer(ITftpTransfer transfer, Stream stream)
        {
            transfer.OnProgress += new TftpProgressHandler(transfer_OnProgress);
            transfer.OnError += new TftpErrorHandler(transfer_OnError);
            transfer.OnFinished += new TftpEventHandler(transfer_OnFinished);

            var operation = TransferOperation.StartNew();
            if (!TransferOperations.TryAdd(transfer, operation))
            {
                TransferOperations[transfer] = operation;
            }

            transfer.Start(stream);
        }

        private static void CancelTransfer(ITftpTransfer transfer, TftpErrorPacket reason)
        {
            OutputTransferStatus(transfer, $"Rejecting transfer: {reason.ErrorMessage}");
            Console.WriteLine();
            transfer.Cancel(reason);

            TransferOperations.Remove(transfer);
        }

        static void transfer_OnError(ITftpTransfer transfer, TftpTransferError error)
        {
            TransferOperations.Remove(transfer);

            if (lastProgressTransfer is not null)
                Console.WriteLine();

            OutputTransferStatus(transfer, $"Error: {error}");
            Console.WriteLine();
        }

        static void transfer_OnFinished(ITftpTransfer transfer)
        {
            TransferOperations.Remove(transfer);

            if (lastProgressTransfer is not null)
                Console.WriteLine();

            OutputTransferStatus(transfer, "Finished");
            Console.WriteLine();
        }

        private static volatile ITftpTransfer? lastProgressTransfer = null;
        static void transfer_OnProgress(ITftpTransfer transfer, TftpTransferProgress progress)
        {
            TransferOperation operation;

            if (!TransferOperations.TryGetValue(transfer, out operation!))
            {
                Debug.Assert(operation is not null);

                TransferOperations.Add(transfer, operation = TransferOperation.StartNew());
            }

            double speed = operation.UpdateTransferedBytes(progress.TransferredBytes);

            long totalBytes = unchecked((uint)progress.TotalBytes);
            double percentage = totalBytes <= 0 ? double.NaN : 100.0 * progress.TransferredBytes / totalBytes;

            Console.Write(lastProgressTransfer == transfer ? '\r' : Environment.NewLine);
            OutputTransferStatus(transfer, $"[{percentage:F2}%] transferred {ToReadableBytes(progress.TransferredBytes)} of {ToReadableBytes(totalBytes)}, {ToReadableBytes(speed):F2}/s ", false);
            lastProgressTransfer = transfer;

            static string ToReadableBytes(double bytes)
            {
                if (bytes <= 1024)
                    return $"{bytes:F1} b";

                bytes /= 1024;
                if (bytes <= 1024)
                    return $"{bytes:F1} Kb";

                bytes /= 1024;
                if (bytes <= 1024)
                    return $"{bytes:F1} Mb";

                bytes /= 1024;
                return $"{bytes:F1} Gb";
            }
        }

        private static void OutputTransferStatus(ITftpTransfer transfer, string message, bool newLine = true)
        {
            string line = $"[{DateTime.Now:HH:mm:ss}] [{transfer.Filename}] {message}";

            if (newLine)
            {
                Console.WriteLine(line);
            }
            else
            {
                Console.Write(line);
            }

            lastProgressTransfer = null;
        }
    }

    class TransferOperation
    {
        private Stopwatch ProgressStopWatch { get; set; } = null!;
        public long LastTransferredBytes { get; private set; }

        public static TransferOperation StartNew() => new TransferOperation
        {
            ProgressStopWatch = Stopwatch.StartNew()
        };

        public double UpdateTransferedBytes(long transferedBytes)
        {
            long deltaBytes = transferedBytes - LastTransferredBytes;
            Debug.Assert(deltaBytes >= 0);

            LastTransferredBytes = transferedBytes;

            Debug.Assert(ProgressStopWatch.IsRunning);

            double elapsedSeconds = ProgressStopWatch.Elapsed.TotalSeconds;
            ProgressStopWatch.Restart();

            if (elapsedSeconds == 0)
            {
                return 0;
            }

            return deltaBytes / elapsedSeconds;
        }
    }
}
