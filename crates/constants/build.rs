use std::fs::File;
use std::io::Write;
use std::process::Command;

fn main() {
    let output = Command::new("date")
        .arg("+%a, %d %b %Y %H:%M:%S %z")
        .env("LC_TIME", "C") // 设置环境变量 LC_TIME 为 C 确保日期输出为英文
        .output()
        .expect("Failed to execute command");

    let build_time = String::from_utf8(output.stdout).unwrap();
    let mut f = File::create("src/build_info.rs").unwrap();
    writeln!(f, "pub const BUILD_TIME: &str = \"{}\";", build_time.trim()).unwrap();
}
