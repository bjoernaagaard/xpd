//! `xpd mcp`: starts the MCP server, the `xpd.mcp` package that the wheel installs beside the
//! binary, with the Python of the same environment.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::process::{Command, ExitCode};

/// The Python that a wheel's environment holds beside the scripts it installs.
const PYTHON: &str = if cfg!(windows) {
    "python.exe"
} else {
    "python"
};
/// Exits 0 when `FastMCP` is installed, without the cost of importing it. The wheel installs it,
/// and a binary built with cargo has no Python beside it.
const HAS_FASTMCP: &str =
    "import importlib.util, sys; sys.exit(importlib.util.find_spec('fastmcp') is None)";
/// The command that installs and starts the server in one step.
const INSTALL: &str = "xpd mcp needs xpd installed as a Python package. Run it as: uvx xpd mcp";
/// Exit code when the server cannot start.
const NOT_STARTED: u8 = 2;

/// Runs `python -m xpd.mcp ARGS...` with `XPD_BIN` set to this binary, and passes on its exit
/// code. The arguments pass through untouched, so `xpd mcp -h` prints the server's help.
pub(crate) fn run(args: &[OsString]) -> ExitCode {
    // An installer may link the binary from another folder, and the Python is beside the target.
    let exe = match env::current_exe().and_then(fs::canonicalize) {
        Ok(exe) => exe,
        Err(e) => {
            eprintln!("xpd: cannot find the xpd binary: {e}");
            return ExitCode::from(NOT_STARTED);
        }
    };
    let python = exe.with_file_name(PYTHON);
    let ready = Command::new(&python).args(["-c", HAS_FASTMCP]).status();
    if !ready.is_ok_and(|status| status.success()) {
        eprintln!("xpd: {INSTALL}");
        return ExitCode::from(NOT_STARTED);
    }
    let mut server = Command::new(&python);
    server
        .args(["-m", "xpd.mcp"])
        .args(args)
        .env("XPD_BIN", &exe);
    start(&mut server)
}

/// Replaces this process with the server, so a client that stops `xpd mcp` stops the server.
#[cfg(unix)]
fn start(server: &mut Command) -> ExitCode {
    use std::os::unix::process::CommandExt;

    let e = server.exec();
    eprintln!("xpd: cannot start the MCP server: {e}");
    ExitCode::from(NOT_STARTED)
}

/// Windows has no exec, so this process waits for the server and passes on its exit code.
#[cfg(not(unix))]
fn start(server: &mut Command) -> ExitCode {
    crate::output::child_exit_code(server.status(), "cannot start the MCP server")
}
