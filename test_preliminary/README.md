# Test artifacts for preliminary

This directory contains test artifacts for the preliminary round of the contest.

The grading scripts and sdcard.img containing test executables are from the contest host. I do not own them. You can find the original files of the grading scripts and test case source code in the contest repository. You have to build the test cases and create the sdcard.img yourself. 

You can use them from this repository on your own risk. I do not provide any guranatee nor warranty.

The visualizer script is written by me and is licensed under the MIT license. It takes the output of the grading script(which is a JSON file) and creates a visual representation of the test results, indluding result for each test case and the overall score.

The visualizer script is provided as is and I do not provide any guarantee nor warranty.

The make file in the root directory of this repository contains the following targets:

- `all` - Build the kernel and generate the correct files for the contest host
- `test` - Simulate the grading process of the contest host and generate a visual representation of the test results
