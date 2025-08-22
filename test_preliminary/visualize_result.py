import json
import sys

TEST_SKIPED = 0
TEST_FAILED = 1
TEST_PASSED = 2

def get_prefix(status: int):
    if status == TEST_FAILED:
        # Red "Failed"
        return "\033[31mFailed\033[0m"
    elif status == TEST_SKIPED:
        # Bright Yellow "Half"
        return "\033[93mSkiped\033[0m"
    elif status == TEST_PASSED:
        # Green "Passed"
        return "\033[32mPassed\033[0m"

def visualize(data):
    total_tests = len(data)
    failed = 0
    skiped = 0
    passed = 0

    total_scores = 0
    score = 0
    for test in data:
        name = test['name']
        num_total = test['all']
        num_passed = test['passed']
        judgements = len(test['results'])

        total_scores += num_total
        score += num_passed

        status = 0

        if judgements == 0:
            skiped += 1
            status = TEST_SKIPED
        elif num_passed == num_total:
            passed += 1
            status = TEST_PASSED
        else:
            failed += 1
            status = TEST_FAILED

        print(f"  {get_prefix(status)} {name} [{num_passed}/{num_total}]")
    
    print()
    print(f"Total tests: {total_tests}")
    # I know this is a typo, but i want to make it aligned
    print(f"    Skiped: {skiped}")
    print(f"    Passed: {passed}")
    print(f"    Failed: {failed}")
    print()
    print(f"Scores: {score}/{total_scores}")
    print()
    print("Raw test result and kernel output will be uploaded to the artifacts")

    return score == total_scores

def read_output(file_name: str):
    with open(file_name, "r") as file:
        return file.read()

if __name__ == '__main__':
    file_name = sys.argv[1]
    output = read_output(file_name)
    data = json.loads(output)
    print(r"                          ___  ____  ")
    print(r"  _ __   ___ _   _  __ _ / _ \/ ___| ")
    print(r" | '_ \ / _ \ | | |/ _` | | | \___ \ ")
    print(r" | | | |  __/ |_| | (_| | |_| |___) |")
    print(r" |_| |_|\___|\__,_|\__, |\___/|____/ ")
    print(r"  _____         _     |_|            ")
    print(r" |_   _|__  ___| |_                  ")
    print(r"   | |/ _ \/ __| __|                 ")
    print(r"   | |  __/\__ \ |_                  ")
    print(r"   |_|\___||___/\__|                 ")
    print("Result:")
    res = visualize(data)

    if not res:
        exit(1)
