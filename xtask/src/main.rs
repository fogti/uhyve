use std::{
	fs,
	io::{Read as _, Seek as _, SeekFrom, Write as _},
	path::PathBuf,
};

use toml_edit::{
	DocumentMut as TDocumentMut, InlineTable as TInlineTable, Item as TItem, Table as TTable,
	value as tvalue,
};

#[derive(clap::Parser)]
struct Cli {
	/// If specified, attempt to undo the modification / command.
	#[arg(short)]
	undo: bool,

	#[command(subcommand)]
	command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
	/// Prepares a Hermit kernel worktree to be used with this `uhyve-interface` version.
	PatchKernel {
		/// The path to the Hermit kernel worktree
		#[arg(short, long)]
		worktree: PathBuf,
	},
}

fn main() {
	let cli = <Cli as clap::Parser>::parse();

	match cli.command {
		Command::PatchKernel { worktree } => {
			let mut cgt = fs::OpenOptions::new()
				.read(true)
				.write(true)
				.open(worktree.join("Cargo.toml"))
				.expect("unable to open kernel's Cargo.toml");
			let mut doc = {
				let mut cgt_content = String::new();
				cgt.read_to_string(&mut cgt_content)
					.expect("unable to read kernel's Cargo.toml");
				cgt_content
					.parse::<TDocumentMut>()
					.expect("unable to parse kernel's Cargo.toml")
			};

			{
				let patch_area = doc["patch"]["crates-io"]
					.or_insert(TItem::Table(TTable::new()))
					.as_table_like_mut()
					.expect(".patch.crates-io isn't a TOML table");
				if cli.undo {
					patch_area.remove("uhyve-interface");
				} else {
					let uhyveif = patch_area
						.entry("uhyve-interface")
						.or_insert(TItem::Value(TInlineTable::new().into()));
					let uhyveif_path = std::env::current_dir()
						.expect("unable to find current working directory")
						.join("uhyve-interface");
					uhyveif["path"] = tvalue(
						uhyveif_path
							.to_str()
							.expect("path to uhyve-interface isn't UTF-8"),
					);
					uhyveif.as_inline_table_mut().map(|t| t.fmt());
				}
			}

			let cgt_content = doc.to_string();
			cgt.seek(SeekFrom::Start(0)).unwrap();
			cgt.set_len(0)
				.expect("unable to truncate kernel's Cargo.toml");
			cgt.write_all(cgt_content.as_bytes())
				.expect("unable to write updated kernel's Cargo.toml");
			cgt.flush()
				.expect("unable to flush updated kernel's Cargo.toml");
			cgt.sync_data().unwrap();
		}
	}
}
