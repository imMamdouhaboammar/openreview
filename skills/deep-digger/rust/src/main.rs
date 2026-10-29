//! Standalone Rust oracle runner; commands require explicit execution opt-in.
use deep_digger_min::{Verdict, ddmin};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct Config {
    input: PathBuf,
    output: PathBuf,
    max_tests: usize,
    repeats: usize,
    timeout: Duration,
    command: Vec<String>,
}

fn parse() -> Result<Config, String> {
    let mut input = None;
    let mut output = None;
    let mut max_tests = 200;
    let mut repeats = 3;
    let mut timeout = Duration::from_secs(15);
    let mut allow_exec = false;
    let mut command = Vec::new();
    let mut args = env::args().skip(1);
    while let Some(item) = args.next() {
        match item.as_str() {
            "--input" => input = args.next().map(PathBuf::from),
            "--output" => output = args.next().map(PathBuf::from),
            "--max-tests" => max_tests = args.next().ok_or("--max-tests value missing")?
                .parse().map_err(|_| "invalid --max-tests")?,
            "--repeats" => repeats = args.next().ok_or("--repeats value missing")?
                .parse().map_err(|_| "invalid --repeats")?,
            "--timeout" => {
                let seconds: u64 = args.next().ok_or("--timeout value missing")?
                    .parse().map_err(|_| "invalid --timeout")?;
                timeout = Duration::from_secs(seconds);
            }
            "--allow-exec" => allow_exec = true,
            "--" => {
                command = args.collect();
                break;
            }
            "--help" => return Err("usage: deep-digger-min --input FILE --output NEWFILE --allow-exec [--repeats 3] [--max-tests 200] [--timeout SECONDS] -- oracle {input}".into()),
            _ => return Err(format!("unknown argument: {item}")),
        }
    }
    let input = input.ok_or("--input required")?;
    let output = output.ok_or("--output required")?;
    if !allow_exec || !(2..=10).contains(&repeats) || max_tests == 0 || timeout.is_zero() {
        return Err("requires --allow-exec, repeats 2..10, positive max-tests and timeout".into());
    }
    if command.is_empty() || !command.iter().any(|x| x.contains("{input}")) {
        return Err("oracle command after -- must contain {input}".into());
    }
    if output.exists()
        || fs::canonicalize(&input).map_err(|x| x.to_string())?
            == output.canonicalize().unwrap_or(output.clone())
    {
        return Err("input must exist; output must be new and different".into());
    }
    Ok(Config {
        input,
        output,
        max_tests,
        repeats,
        timeout,
        command,
    })
}

fn verdict(command: &[String], candidate: &Path, timeout: Duration) -> Verdict {
    let argv: Vec<String> = command
        .iter()
        .map(|arg| arg.replace("{input}", &candidate.to_string_lossy()))
        .collect();
    let result = Command::new(&argv[0])
        .args(&argv[1..])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = result else {
        return Verdict::Unresolved;
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return match status.code() {
                    Some(0) => Verdict::Pass,
                    Some(1) => Verdict::Fail,
                    _ => Verdict::Unresolved,
                };
            }
            Ok(None) if start.elapsed() < timeout => std::thread::sleep(Duration::from_millis(25)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Verdict::Unresolved;
            }
        }
    }
}

fn run() -> Result<(), String> {
    let config = parse()?;
    let text = fs::read_to_string(&config.input).map_err(|x| x.to_string())?;
    let lines: Vec<String> = text.split_inclusive('\n').map(str::to_owned).collect();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|x| x.to_string())?
        .as_nanos();
    let candidate = env::temp_dir().join(format!("deep-digger-{}-{}", std::process::id(), nanos));
    let oracle = |chunks: &[String]| {
        let mut output = match fs::File::create(&candidate) {
            Ok(file) => file,
            Err(_) => return Verdict::Unresolved,
        };
        for chunk in chunks {
            if output.write_all(chunk.as_bytes()).is_err() {
                return Verdict::Unresolved;
            }
        }
        drop(output);
        let outcomes: Vec<_> = (0..config.repeats)
            .map(|_| verdict(&config.command, &candidate, config.timeout))
            .collect();
        if outcomes.iter().all(|x| *x == outcomes[0]) {
            outcomes[0]
        } else {
            Verdict::Unresolved
        }
    };
    let result = ddmin(&lines, config.max_tests, oracle);
    let _ = fs::remove_file(&candidate);
    let result = result?;
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&config.output)
        .map_err(|x| x.to_string())?;
    for line in &result.lines {
        output
            .write_all(line.as_bytes())
            .map_err(|x| x.to_string())?;
    }
    println!(
        "retained_lines={} tests={} one_minimal_verified={}",
        result.lines.len(),
        result.tests,
        result.complete
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Deep Digger Rust: {error}");
        std::process::exit(2);
    }
}
