//! Disposable source layout for real manager processes; never log in the build checkout.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

pub(crate) fn binaries() -> PathBuf {
    static BUILD: OnceLock<()> = OnceLock::new();
    BUILD.get_or_init(|| {
        assert!(Command::new(env!("CARGO"))
            .args([
                "build",
                "--manifest-path",
                concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"),
                "--locked",
                "--release",
                "--bins"
            ])
            .status()
            .unwrap()
            .success());
    });
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release")
}

pub(crate) struct Checkout {
    pub(crate) root: tempfile::TempDir,
    pub(crate) executable: PathBuf,
}

impl Checkout {
    pub(crate) fn new() -> Self {
        let binaries = binaries();
        let root = tempfile::tempdir().unwrap();
        for directory in [
            "core/src",
            "profiles",
            "skills",
            "mcp/servers",
            "core/target/release",
        ] {
            fs::create_dir_all(root.path().join(directory)).unwrap();
        }
        let executable = root.path().join("core/target/release/ai-profile");
        copy_executable(&binaries.join("ai-profile"), &executable);
        Self { root, executable }
    }

    pub(crate) fn with_assets(binaries: &Path) -> Self {
        let checkout = Self::new();
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        for directory in ["core", "profiles", "skills", "mcp"] {
            copy_tree(
                &source.join(directory),
                &checkout.root.path().join(directory),
            );
        }
        for name in [
            "ai-profile-mcp",
            "playwright-launcher",
            "restore-updraft",
            "test-inventory",
        ] {
            copy_executable(
                &binaries.join(name),
                &checkout.executable.parent().unwrap().join(name),
            );
        }
        checkout
    }
}

pub(crate) fn copy_executable(source: &Path, destination: &Path) {
    // Keep writable executable FDs out of this multithreaded test process:
    // concurrent forks could inherit one until exec and cause ETXTBSY even
    // after fs::copy returns. The copy child has closed all FDs when it exits.
    assert!(Command::new("cp")
        .arg("--")
        .arg(source)
        .arg(destination)
        .status()
        .unwrap()
        .success());
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == "target" {
            continue;
        }
        let target = destination.join(entry.file_name());
        let kind = entry.file_type().unwrap();
        if kind.is_symlink() {
            std::os::unix::fs::symlink(fs::read_link(entry.path()).unwrap(), target).unwrap();
        } else if kind.is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}
