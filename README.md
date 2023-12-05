# Multi-log Consumer
The rust crate is design to solve the problem of running experiments across different processes. These processes will output to various log files, which are difficult to sync up if the logs do not have a timestamp. This crate tails a list of arbitary logs files, timestamps the entries and exports to a central file.

To sync up logs, a sampling rate must be set. This is a lossy operation is the sample rate is below a files update rate; this will also conversely duplicate files, however a future todo is to have the option to disable this.

```sh
Usage: log-consumer [OPTIONS] --log-files <LOG_FILES> --output-file <OUTPUT_FILE>

Options:
  -l, --log-files <LOG_FILES>      Log files to tail (comma separated)
  -o, --output-file <OUTPUT_FILE>  Output file
  -p, --precision <PRECISION>      Sampling rate in milliseconds (default: 1000)
  -v...                            Verbose mode (-v)
  -h, --help                       Print help
  -V, --version                    Print version
```

## Sample usage
The program does not take in a time to run, instead it exists on a Ctrl-C command.
```sh
log-consumer --log-files /tmp/process_one.log,/tmp/process_two.log -o output.csv -p 500 -v
Beginning Listen...
source: /tmp/process_two.log, line: Two 46
source: /tmp/process_one.log, line: One 89
source: /tmp/process_one.log, line: One 90
source: /tmp/process_two.log, line: Two 47
source: /tmp/process_one.log, line: One 91
source: /tmp/process_one.log, line: One 92
source: /tmp/process_two.log, line: Two 48
source: /tmp/process_one.log, line: One 93
source: /tmp/process_one.log, line: One 94
source: /tmp/process_two.log, line: Two 49
^C
Finished Listen...
Generating Report @ 500ms...
CSV exported to file: output.csv
```