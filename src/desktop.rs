// Starts Voicenux, or brings it to the front on a page: `voicenux open settings`.
// Voicenux runs once, so a second start only opens the page in the running app.
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::Res;

const APP_ID: &str = "io.github.voicenux.Voicenux";

fn in_path(name: &str) -> Option<PathBuf> {
	let path = std::env::var_os("PATH")?;
	std::env::split_paths(&path)
		.filter(|dir| !dir.as_os_str().is_empty())
		.map(|dir| dir.join(name))
		.find(|file| std::fs::metadata(file).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0))
}

fn has_flatpak() -> bool {
	let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
	let places = [home.join(".local/share/flatpak/app").join(APP_ID), Path::new("/var/lib/flatpak/app").join(APP_ID)];
	in_path("flatpak").is_some() && places.iter().any(|dir| dir.exists())
}

/// How to run Voicenux with arguments, or None when it is not installed.
pub fn voicenux_command(args: &[&str]) -> Option<Vec<String>> {
	let mut command = match in_path("voicenux") {
		Some(bin) => vec![bin.to_string_lossy().into_owned()],
		None if has_flatpak() => vec!["flatpak".into(), "run".into(), APP_ID.into()],
		None => return None,
	};
	command.extend(args.iter().map(|a| a.to_string()));
	Some(command)
}

pub fn open_voicenux(page: &str) -> Res<()> {
	let command = voicenux_command(&["open", page]).ok_or("Voicenux is not installed")?;
	// A group of its own, so Voicenux keeps running when OpenDeck stops the plugin.
	let child = tokio::process::Command::new(&command[0])
		.args(&command[1..])
		.stdin(Stdio::null())
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.process_group(0)
		.spawn();
	// Wait for it in the background, so it does not stay a zombie.
	if let Ok(mut child) = child {
		tokio::task::spawn_local(async move {
			let _ = child.wait().await;
		});
	}
	Ok(())
}

pub fn installed() -> bool {
	voicenux_command(&[]).is_some()
}
