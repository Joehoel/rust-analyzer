use std::{
    env,
    fs::File,
    io,
    path::{Path, PathBuf},
};

use flate2::{write::GzEncoder, Compression};
use xshell::{cmd, Shell};

use crate::{date_iso, flags, project_root};

const VERSION_STABLE: &str = "0.3";
const VERSION_NIGHTLY: &str = "0.4";
const VERSION_DEV: &str = "0.5"; // keep this one in sync with `package.json`

impl flags::Dist {
    pub(crate) fn run(self, sh: &Shell) -> anyhow::Result<()> {
        let stable = sh.var("GITHUB_REF").unwrap_or_default().as_str() == "refs/heads/release";

        let project_root = project_root();
        let target = Target::get(&project_root);
        let dist = project_root.join("dist");
        sh.remove_path(&dist)?;
        sh.create_dir(&dist)?;

        let release_channel = if stable { "stable" } else { "nightly" };
        dist_server(sh, release_channel, &target)?;

        if let Some(patch_version) = self.client_patch_version {
            let version = if stable {
                format!("{}.{}", VERSION_STABLE, patch_version)
            } else {
                // A hack to make VS Code prefer nightly over stable.
                format!("{}.{}", VERSION_NIGHTLY, patch_version)
            };
            let release_tag = if stable { date_iso(sh)? } else { "nightly".to_string() };
            dist_client(sh, &version, &release_tag, &target)?;
        }
        Ok(())
    }
}

fn dist_client(
    sh: &Shell,
    version: &str,
    release_tag: &str,
    target: &Target,
) -> anyhow::Result<()> {
    let bundle_path = Path::new("editors").join("code").join("server");
    sh.create_dir(&bundle_path)?;
    sh.copy_file(&target.server_path, &bundle_path)?;
    if let Some(symbols_path) = &target.symbols_path {
        sh.copy_file(symbols_path, &bundle_path)?;
    }

    let _d = sh.push_dir("./editors/code");

    let mut patch = Patch::new(sh, "./package.json")?;
    patch
        .replace(
            &format!(r#""version": "{}.0-dev""#, VERSION_DEV),
            &format!(r#""version": "{}""#, version),
        )
        .replace(r#""releaseTag": null"#, &format!(r#""releaseTag": "{}""#, release_tag))
        .replace(r#""$generated-start": {},"#, "")
        .replace(",\n                \"$generated-end\": {}", "")
        .replace(r#""enabledApiProposals": [],"#, r#""#);
    patch.commit(sh)?;

    Ok(())
}

fn dist_server(sh: &Shell, release_channel: &str, target: &Target) -> anyhow::Result<()> {
    let _e = sh.push_env("RUST_ANALYZER_CHANNEL", release_channel);
    let _e = sh.push_env("CARGO_PROFILE_RELEASE_LTO", "thin");

    // Uncomment to enable debug info for releases. Note that:
    //   * debug info is split on windows and macs, so it does nothing for those platforms,
    //   * on Linux, this blows up the binary size from 8MB to 43MB, which is unreasonable.
    // let _e = sh.push_env("CARGO_PROFILE_RELEASE_DEBUG", "1");

    if target.name.contains("-linux-") {
        env::set_var("CC", "clang");
    }

    let target_name = &target.name;
    cmd!(sh, "cargo build --manifest-path ./crates/rust-analyzer/Cargo.toml --bin rust-analyzer --target {target_name} --release").run()?;

    let dst = Path::new("dist").join(&target.artifact_name);
    gzip(&target.server_path, &dst.with_extension("gz"))?;

    Ok(())
}

fn gzip(src_path: &Path, dest_path: &Path) -> anyhow::Result<()> {
    let mut encoder = GzEncoder::new(File::create(dest_path)?, Compression::best());
    let mut input = io::BufReader::new(File::open(src_path)?);
    io::copy(&mut input, &mut encoder)?;
    encoder.finish()?;
    Ok(())
}

struct Target {
    name: String,
    server_path: PathBuf,
    symbols_path: Option<PathBuf>,
    artifact_name: String,
}

impl Target {
    fn get(project_root: &Path) -> Self {
        let name = match env::var("RA_TARGET") {
            Ok(target) => target,
            _ => {
                if cfg!(target_os = "linux") {
                    "x86_64-unknown-linux-gnu".to_string()
                } else if cfg!(target_os = "windows") {
                    "x86_64-pc-windows-msvc".to_string()
                } else if cfg!(target_os = "macos") {
                    "x86_64-apple-darwin".to_string()
                } else {
                    panic!("Unsupported OS, maybe try setting RA_TARGET")
                }
            }
        };
        let out_path = project_root.join("target").join(&name).join("release");
        let (exe_suffix, symbols_path) = if name.contains("-windows-") {
            (".exe".into(), Some(out_path.join("rust_analyzer.pdb")))
        } else {
            (String::new(), None)
        };
        let server_path = out_path.join(format!("rust-analyzer{}", exe_suffix));
        let artifact_name = format!("rust-analyzer-{}{}", name, exe_suffix);
        Self { name, server_path, symbols_path, artifact_name }
    }
}

struct Patch {
    path: PathBuf,
    original_contents: String,
    contents: String,
}

impl Patch {
    fn new(sh: &Shell, path: impl Into<PathBuf>) -> anyhow::Result<Patch> {
        let path = path.into();
        let contents = sh.read_file(&path)?;
        Ok(Patch { path, original_contents: contents.clone(), contents })
    }

    fn replace(&mut self, from: &str, to: &str) -> &mut Patch {
        assert!(self.contents.contains(from));
        self.contents = self.contents.replace(from, to);
        self
    }

    fn commit(&self, sh: &Shell) -> anyhow::Result<()> {
        sh.write_file(&self.path, &self.contents)?;
        Ok(())
    }
}

impl Drop for Patch {
    fn drop(&mut self) {
        // FIXME: find a way to bring this back
        let _ = &self.original_contents;
        // write_file(&self.path, &self.original_contents).unwrap();
    }
}
