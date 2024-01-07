use std::fmt::Display;
use std::io;
use std::io::Write;
use std::path::PathBuf;

const CRATE_VERSION: &str = include_cargo_toml::include_toml!("package"."version");

/// Run the Tailwind CLI with the given arguments.
///
/// ```
/// let args = vec!["--input", "src/main.css", "--output", "target/built.css"];
/// tailwind_cli::run(args).expect("Running Tailwind CLI failed.");
/// ```
pub fn run<Args>(args: Args) -> Result<TailwindCliOutput, TailwindCliError>
where
    Args: IntoIterator,
    Args::Item: Into<std::ffi::OsString>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    println!("Running Tailwind CLI with args: {:?}", args);

    let path_to_cli_executable = get_cli_executable_file()?;
    println!("Got CLI executable file: {:?}", path_to_cli_executable);
    println!("Executing CLI executable...");
    let output = duct::cmd(&path_to_cli_executable, args)
        .stderr_capture()
        .stdout_capture()
        .unchecked()
        .run()
        .map_err(TailwindCliError::CouldntInvokeTailwindCli)?;

    let (stdout, stderr) = get_stdout_and_stderr_from_process_output(&output);

    println!("CLI executable finished executing.");
    println!("CLI executable stdout: {}", &stdout);
    println!("CLI executable stderr: {}", &stderr);

    std::fs::remove_file(path_to_cli_executable)
        .map_err(TailwindCliError::CouldntDeleteTemporaryFile)?;
    println!("Deleted temporary file.");

    if !output.status.success() {
        println!("CLI executable returned an error.");
        let error = TailwindCliError::TailwindCliReturnedAnError { stdout, stderr };
        return Err(error);
    }

    println!("CLI executable returned successfully.");
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

fn get_cli_executable_file() -> Result<PathBuf, TailwindCliError> {
    let platform = guess_platform();
    println!("Guessed platform: {:?}", platform);
    let cli_executable_bytes = get_cli_executable_bytes(&platform);
    println!(
        "Got CLI executable bytes: {} bytes",
        cli_executable_bytes.len()
    );

    // We use a UUID in case multiple builds are running at the same time.
    let uuid = uuid::Uuid::new_v4().to_string();
    let temp_file_name = format!("tailwindcss-{}-v{}-{}", platform, CRATE_VERSION, uuid);
    let temp_file_path = std::env::current_dir()
        .map_err(TailwindCliError::CouldntSaveCliExecutableToTemporaryFile)?
        .join("target")
        .join(temp_file_name);

    let mut temp_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&temp_file_path)
        .map_err(TailwindCliError::CouldntSaveCliExecutableToTemporaryFile)?;
    // let mut temp_file =
    //     NamedTempFile::new().map_err(TailwindCliError::CouldntSaveCliExecutableToTemporaryFile)?;
    println!("Created temporary file: {:?}", &temp_file_path);

    temp_file
        .write_all(cli_executable_bytes)
        .map_err(TailwindCliError::CouldntSaveCliExecutableToTemporaryFile)?;
    println!("Wrote CLI executable bytes to temporary file.");

    // Make the file executable. This isn't supported on Windows, so we skip it.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = temp_file
            .metadata()
            .map_err(TailwindCliError::CouldntSaveCliExecutableToTemporaryFile)?
            .permissions();
        // 755 - owner can read/write/execute, group/others can read/execute.
        permissions.set_mode(0o755);
        temp_file
            .set_permissions(permissions)
            .map_err(TailwindCliError::CouldntSaveCliExecutableToTemporaryFile)?;
        println!("Made temporary file executable.");
    }

    // Make sure the file is closed and written to disk.
    temp_file
        .sync_all()
        .map_err(TailwindCliError::CouldntSaveCliExecutableToTemporaryFile)?;
    drop(temp_file);

    Ok(temp_file_path)
}

#[derive(Debug)]
enum Platform {
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

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Platform::MacOsArm64 => "macos-arm64",
            Platform::MacOsX64 => "macos-x64",
            Platform::LinuxArm64 => "linux-arm64",
            Platform::LinuxArmv7 => "linux-armv7",
            Platform::LinuxX64 => "linux-x64",
            Platform::WindowsArm64 => "windows-arm64",
            Platform::WindowsX64 => "windows-x64",
        };
        write!(f, "{}", name)
    }
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

fn get_cli_executable_bytes(platform: &Platform) -> &'static [u8] {
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
    CouldntDeleteTemporaryFile(io::Error),
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
            TailwindCliError::CouldntDeleteTemporaryFile(error) => {
                write!(
                    f,
                    "Couldn't delete Tailwind CLI executable temporary file: {}",
                    error
                )
            }
        }
    }
}

impl std::error::Error for TailwindCliError {}

#[cfg(test)]
mod tests {

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
        let built_css_path = "target/built.css";

        let _ignore_errors = std::fs::remove_file(built_css_path);

        let args = vec!["--input", "src/main.css", "--output", built_css_path];
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
        let built_css_path = "target/built.css";

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
