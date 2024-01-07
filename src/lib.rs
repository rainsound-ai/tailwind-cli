use std::fmt::Display;
use std::io;
use std::io::Write;
use tempfile::NamedTempFile;

/// Equivalent to running `tailwindcss` in the terminal.
pub fn run<Args>(args: Args) -> Result<TailwindCliOutput, TailwindCliError>
where
    Args: IntoIterator,
    Args::Item: Into<std::ffi::OsString>,
{
    // Gotcha: Dropping this file will delete it, so we need to keep it alive.
    // Be careful about giving away ownership of this variable.
    // Details here: https://docs.rs/tempfile/latest/tempfile/#early-drop-pitfall
    let cli_executable_file = get_cli_executable_file()?;
    let path_to_cli_executable = cli_executable_file.path();
    let output = duct::cmd(path_to_cli_executable, args)
        .stderr_capture()
        .stdout_capture()
        .unchecked()
        .run()
        .map_err(TailwindCliError::CouldntInvokeTailwindCli)?;

    if !output.status.success() {
        let (stdout, stderr) = get_stdout_and_stderr_from_process_output(&output);
        let error = TailwindCliError::TailwindCliReturnedAnError { stdout, stderr };
        return Err(error);
    }

    let output = TailwindCliOutput::new(output);
    Ok(output)
}

#[derive(Debug)]
pub struct TailwindCliOutput {
    stdout: String,
    stderr: String,
}

impl TailwindCliOutput {
    fn new(process_output: std::process::Output) -> Self {
        let (stdout, stderr) = get_stdout_and_stderr_from_process_output(&process_output);
        Self { stdout, stderr }
    }

    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    pub fn stderr(&self) -> &str {
        &self.stderr
    }
}

fn get_stdout_and_stderr_from_process_output(
    process_output: &std::process::Output,
) -> (String, String) {
    let stdout = String::from_utf8_lossy(&process_output.stdout)
        .trim()
        .to_string();

    let stderr = String::from_utf8_lossy(&process_output.stderr)
        .trim()
        .to_string();

    (stdout, stderr)
}

fn get_cli_executable_file() -> Result<NamedTempFile, TailwindCliError> {
    let platform = guess_platform();
    let cli_executable_bytes = get_cli_executable_bytes(platform);

    let mut temp_file =
        NamedTempFile::new().map_err(TailwindCliError::CouldntSaveCliExecutableToTemporaryFile)?;

    temp_file
        .write_all(cli_executable_bytes)
        .map_err(TailwindCliError::CouldntSaveCliExecutableToTemporaryFile)?;

    // Make the file executable. This isn't supported on Windows, so we skip it.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let file_handle = temp_file.as_file_mut();
        let mut permissions = file_handle
            .metadata()
            .map_err(TailwindCliError::CouldntSaveCliExecutableToTemporaryFile)?
            .permissions();
        // 755 - owner can read/write/execute, group/others can read/execute.
        permissions.set_mode(0o755);
        file_handle
            .set_permissions(permissions)
            .map_err(TailwindCliError::CouldntSaveCliExecutableToTemporaryFile)?;
    }

    Ok(temp_file)
}

pub enum Platform {
    // macOS
    MacOsArm64,
    MacOsX64,

    // Linux
    LinuxArm64,
    LinuxArmv7,
    LinuxX64,

    // Windows
    WindowsArm64,
    WindowsX64,
}

fn guess_platform() -> Platform {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match os {
        "macos" => match arch {
            "x86_64" => Platform::MacOsX64,
            "aarch64" => Platform::MacOsArm64,
            _ => panic!("Unsupported architecture: {}", arch),
        },
        "linux" => match arch {
            "x86_64" => Platform::LinuxX64,
            "aarch64" => Platform::LinuxArm64,
            "armv7" => Platform::LinuxArmv7,
            _ => panic!("Unsupported architecture: {}", arch),
        },
        "windows" => match arch {
            "x86_64" => Platform::WindowsX64,
            "aarch64" => Platform::WindowsArm64,
            _ => panic!("Unsupported architecture: {}", arch),
        },
        _ => panic!("Unsupported OS: {}", os),
    }
}

fn get_cli_executable_bytes(platform: Platform) -> &'static [u8] {
    match platform {
        Platform::MacOsArm64 => include_bytes!("./tailwindcss-macos-arm64"),
        Platform::MacOsX64 => include_bytes!("./tailwindcss-macos-x64"),
        Platform::LinuxArm64 => include_bytes!("./tailwindcss-linux-arm64"),
        Platform::LinuxArmv7 => include_bytes!("./tailwindcss-linux-armv7"),
        Platform::LinuxX64 => include_bytes!("./tailwindcss-linux-x64"),
        Platform::WindowsArm64 => include_bytes!("./tailwindcss-windows-arm64.exe"),
        Platform::WindowsX64 => include_bytes!("./tailwindcss-windows-x64.exe"),
    }
}

#[derive(Debug)]
pub enum TailwindCliError {
    TailwindCliReturnedAnError { stdout: String, stderr: String },
    CouldntInvokeTailwindCli(io::Error),
    CouldntSaveCliExecutableToTemporaryFile(io::Error),
}

impl Display for TailwindCliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TailwindCliError::TailwindCliReturnedAnError { stdout, stderr } => {
                write!(f, "Tailwind CLI returned an error:\n\n")?;
                write!(f, "stdout:\n{}\n\n", stdout)?;
                write!(f, "stderr:\n{}\n\n", stderr)?;
                Ok(())
            }
            TailwindCliError::CouldntInvokeTailwindCli(error) => {
                write!(f, "Couldn't invoke Tailwind CLI: {}", error)
            }
            TailwindCliError::CouldntSaveCliExecutableToTemporaryFile(error) => {
                write!(
                    f,
                    "Couldn't save Tailwind CLI executable to temporary file: {}",
                    error
                )
            }
        }
    }
}

impl std::error::Error for TailwindCliError {}

#[cfg(test)]
mod tests {
    const CRATE_VERSION: &str = include_cargo_toml::include_toml!("package"."version");

    use super::*;

    #[test]
    fn version_is_correct() {
        let args = vec!["--help"];
        let output = run(&args).expect("Couldn't run `tailwindcss --help`.");
        let stdout = output.stdout();

        // Our crate versions are a string like "3.4.1-0". 3.4.1 is the Tailwind
        // version. 0 is the version of this crate, useful if we need to release
        // a new version of this crate without changing the Tailwind version.
        //
        // Strip everything after the first dash to get the Tailwind version.
        let expected_version = CRATE_VERSION.split('-').next().unwrap();
        let expected_version = format!("tailwindcss v{}", expected_version);
        println!("Command stdout: {}", &stdout);
        println!("Expected version: {}", &expected_version);
        assert!(stdout.contains(&expected_version));
    }

    #[test]
    fn built_css_has_expected_classes() {
        let built_css_path = "target/built_test.css";

        let _ignore_errors = std::fs::remove_file(built_css_path);

        let args = vec!["--input", "src/test.css", "--output", built_css_path];
        run(&args).expect("Couldn't run `tailwindcss`.");

        let font_bold_declaration = ".font-bold {
  font-weight: 700;
}";

        let built_css =
            std::fs::read_to_string(built_css_path).expect("Couldn't read built CSS file.");

        assert!(built_css.contains(font_bold_declaration));
    }

    #[test]
    fn input_file_not_found() {
        let built_css_path = "target/built_test.css";

        let _ignore_errors = std::fs::remove_file(built_css_path);

        let args = vec![
            "--input",
            "src/doesnt_exist.css",
            "--output",
            built_css_path,
        ];
        let result = run(&args);

        if let Err(TailwindCliError::TailwindCliReturnedAnError { stdout, stderr }) = result {
            assert!(stdout.is_empty());
            assert!(stderr.contains("Specified input file src/doesnt_exist.css does not exist."));
        } else {
            panic!(
                "Expected TailwindCliReturnedAnError error, got {:?}",
                result
            );
        }
    }
}
