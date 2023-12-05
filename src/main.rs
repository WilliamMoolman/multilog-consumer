use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Utc};
use clap::Parser;
use itertools::Itertools;
use linemux::MuxedLines;
use std::fs;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Log files to tail (comma separated)
    #[arg(required = true, short, long, value_delimiter = ',')]
    log_files: Vec<String>,

    /// Output file
    #[arg(required = true, short, long)]
    output_file: String,

    /// Sampling rate in milliseconds (default: 1000)
    #[arg(short, long)]
    precision: Option<u64>,

    /// Verbose mode (-v)
    #[arg(short, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Clone, Debug)]
struct LogLine {
    line: String,
    timestamp: DateTime<Utc>,
}

#[derive(Debug)]
struct Log {
    lines: VecDeque<LogLine>,
}

impl Log {
    fn add_line(&mut self, line: String) {
        self.lines.push_back(LogLine {
            line,
            timestamp: Utc::now(),
        });
    }

    fn get_start_time(&self) -> Result<DateTime<Utc>, String> {
        if self.lines.len() > 0 {
            Ok(self.lines[0].timestamp)
        } else {
            Err("No lines in log!".to_string())
        }
    }

    fn get_end_time(&self) -> Result<DateTime<Utc>, String> {
        if self.lines.len() > 0 {
            Ok(self.lines[self.lines.len() - 1].timestamp)
        } else {
            Err("No lines in log!".to_string())
        }
    }
}

struct Report {
    headers: Vec<String>,
    rows: Vec<Vec<LogLine>>,
}

impl Report {
    fn generate(
        logs: &mut HashMap<String, Log>,
        precision: chrono::Duration,
    ) -> Result<Report, String> {
        println!("Generating Report @ {}ms...", precision.num_milliseconds());

        let mut time = logs
            .iter()
            .map(|item| item.1.get_start_time())
            .fold_ok(chrono::DateTime::<Utc>::MIN_UTC, chrono::DateTime::max)?;

        let end_time = logs
            .iter()
            .map(|item| item.1.get_end_time())
            .fold_ok(chrono::DateTime::<Utc>::MAX_UTC, chrono::DateTime::min)?;

        let files: Vec<String> = logs.keys().cloned().collect();
        let mut logs_list: Vec<&mut Log> = logs.values_mut().collect();

        let mut rows: Vec<Vec<LogLine>> = Vec::new();

        while time <= end_time {
            let mut row = Vec::new();
            for (i, log) in logs_list.iter_mut().enumerate() {
                let mut current_logline = None;
                while log.lines[0].timestamp <= time {
                    current_logline = log.lines.pop_front();
                }
                match current_logline {
                    Some(logline) => row.push(logline),
                    None => {
                        row.push(rows.last().expect("Log should have previous line!")[i].clone())
                    }
                }
            }
            rows.push(row);
            time += precision;
        }

        Ok(Report {
            headers: files,
            rows,
        })
    }

    fn generate_csv_content(&self) -> String {
        let mut csv_content = String::new();
        csv_content.push_str(&self.headers.join(","));
        csv_content.push_str("\n");
        for row in &self.rows {
            csv_content.push_str(
                &row.iter()
                    .map(|logline| &logline.line)
                    .map(|logline| logline.as_str())
                    .collect::<Vec<&str>>()
                    .join(","),
            );
            csv_content.push_str("\n");
        }
        csv_content
    }

    fn export_to_csv(&self, filename: String) {
        let csv_content = self.generate_csv_content();

        if let Err(err) = fs::write(&filename, csv_content) {
            eprintln!("Failed to write to file {}: {}", filename, err);
        } else {
            println!("CSV exported to file: {}", filename);
        }
    }
}

async fn consume_logs(
    mut lines: MuxedLines,
    log_outputs: &mut HashMap<String, Log>,
    display: bool,
) {
    println!("Beginning Listen...");
    loop {
        tokio::select! {
            Ok(Some(line)) = lines.next_line() => {

                if display { println!("source: {}, line: {}", line.source().display(), line.line()) };
                let source = line.source().to_str().unwrap().to_string();
                log_outputs.get_mut(&source).unwrap().add_line(line.line().to_string());
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nFinished Listen...");
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let mut lines = MuxedLines::new()?;
    let mut output: HashMap<String, Log> = HashMap::new();

    // Register some files to be tailed, whether they currently exist or not.
    for file in args.log_files {
        lines.add_file(&file).await?;
        output.insert(
            file.clone(),
            Log {
                lines: VecDeque::new(),
            },
        );
    }

    consume_logs(lines, &mut output, args.verbose > 0).await;

    match Report::generate(
        &mut output,
        chrono::Duration::milliseconds(args.precision.unwrap_or(1000).try_into().unwrap()),
    ) {
        Ok(report) => {
            report.export_to_csv(args.output_file);
            Ok(())
        }
        Err(err) => {
            eprintln!("Failed to generate report: {}", err);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to generate report",
            ))
        }
    }
}
