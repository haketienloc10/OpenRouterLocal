use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use nix::{
    sys::signal::{kill, Signal},
    unistd::Pid,
};

const LOG_DIR: &str = "logs";
const PID_FILE: &str = "server.pid";
const LOG_FILE: &str = "server.log";

fn logs_dir() -> PathBuf {
    PathBuf::from(LOG_DIR)
}

fn pid_path() -> PathBuf {
    logs_dir().join(PID_FILE)
}

fn log_path() -> PathBuf {
    logs_dir().join(LOG_FILE)
}

fn ensure_logs_dir() -> Result<()> {
    fs::create_dir_all(logs_dir()).context("failed to create logs directory")
}

pub fn read_pid() -> Option<u32> {
    let content = fs::read_to_string(pid_path()).ok()?;
    content.trim().parse::<u32>().ok()
}

pub fn is_alive(pid: u32) -> bool {
    PathBuf::from(format!("/proc/{pid}")).exists()
}

fn send_signal(pid: u32, signal: Signal) -> Result<()> {
    kill(Pid::from_raw(pid as i32), signal)
        .with_context(|| format!("failed to send {signal:?} to pid={pid}"))
}

fn write_pid(pid: u32) -> Result<()> {
    fs::write(pid_path(), format!("{pid}\n")).context("failed to write pid file")
}

pub fn start_background() -> Result<()> {
    ensure_logs_dir()?;

    if let Some(pid) = read_pid() {
        if is_alive(pid) {
            println!("already running (pid={pid})");
            return Ok(());
        }
    }

    let exe = std::env::current_exe().context("failed to resolve current executable")?;
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
        .context("failed to open log file")?;
    let log_err = log.try_clone().context("failed to clone log file handle")?;
    let devnull = File::open("/dev/null").context("failed to open /dev/null")?;

    let child = Command::new(exe)
        .arg("serve")
        .stdin(Stdio::from(devnull))
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .spawn()
        .context("failed to spawn background server")?;

    let pid = child.id();
    write_pid(pid)?;

    println!("started (pid={pid})");
    Ok(())
}

pub fn stop_background() -> Result<()> {
    let path = pid_path();
    if !path.exists() {
        println!("not running");
        return Ok(());
    }

    let Some(pid) = read_pid() else {
        let _ = fs::remove_file(&path);
        println!("not running");
        return Ok(());
    };

    if !is_alive(pid) {
        let _ = fs::remove_file(&path);
        println!("not running");
        return Ok(());
    }

    send_signal(pid, Signal::SIGTERM)?;

    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if !is_alive(pid) {
            let _ = fs::remove_file(&path);
            println!("stopped");
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }

    if is_alive(pid) {
        send_signal(pid, Signal::SIGKILL)?;
        let kill_deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < kill_deadline {
            if !is_alive(pid) {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    let _ = fs::remove_file(&path);
    if is_alive(pid) {
        anyhow::bail!("failed to stop process pid={pid}");
    }

    println!("stopped");
    Ok(())
}

pub fn restart_background() -> Result<()> {
    stop_background()?;
    start_background()
}

pub fn logs(lines: usize, follow: bool) -> Result<()> {
    ensure_logs_dir()?;
    tail_file(lines)?;
    if follow {
        follow_file()?;
    }
    Ok(())
}

fn tail_file(lines: usize) -> Result<()> {
    let path = log_path();
    if !path.exists() {
        println!("log file not found: {}", path.display());
        return Ok(());
    }

    let file = File::open(&path).with_context(|| format!("failed to open {}", path.display()))?;
    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader.lines().collect::<std::io::Result<Vec<_>>>()?;

    let start = all_lines.len().saturating_sub(lines);
    for line in &all_lines[start..] {
        println!("{line}");
    }

    Ok(())
}

fn follow_file() -> Result<()> {
    let path = log_path();

    let mut offset = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    loop {
        match File::open(&path) {
            Ok(mut file) => {
                let len = file.metadata().map(|m| m.len()).unwrap_or(0);
                if len < offset {
                    offset = 0;
                }

                file.seek(SeekFrom::Start(offset))?;
                let mut buf = String::new();
                file.read_to_string(&mut buf)?;
                if !buf.is_empty() {
                    print!("{buf}");
                    std::io::stdout().flush()?;
                    offset = len;
                }
            }
            Err(_) => {
                offset = 0;
            }
        }

        thread::sleep(Duration::from_millis(500));
    }
}
