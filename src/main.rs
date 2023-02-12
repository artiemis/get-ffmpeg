use std::{
    fs::{self, create_dir_all, remove_dir_all, remove_file, rename, File},
    io::{copy, Write},
    path::Path,
};

use anyhow::{Context, Ok, Result};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use ureq;
use winreg::{enums::*, RegKey};
use zip::ZipArchive;

const DEFAULT_DIR: &str = "C:\\ffmpeg";
const DOWNLOAD_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";

fn get_input(msg: &str, default: &str) -> Result<String> {
    print!("{msg}");
    std::io::stdout().flush()?;
    let mut input = String::new();

    std::io::stdin().read_line(&mut input)?;

    input = input.trim().to_owned();

    if input.is_empty() {
        return Ok(default.to_owned());
    }

    Ok(input)
}

fn download_url(url: &str) -> Result<String> {
    println!("\nDownloading '{url}'...");

    let res = ureq::get(url).call()?;
    let filename = res
        .get_url()
        .split("/")
        .last()
        .context("Failed to extract filename from URL.")?
        .to_owned();

    let mut file = File::create(&filename)?;
    let bar = ProgressBar::new(res.header("Content-Length").unwrap_or("0").parse()?);
    bar.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {msg} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));
    let mut reader = bar.wrap_read(res.into_reader());
    bar.set_message("downloading");

    copy(&mut reader, &mut file)?;
    bar.finish_with_message("done");

    Ok(filename)
}

fn extract_zip(filename: &str) -> Result<()> {
    println!("Extracting '{filename}' to current directory...");

    let file = File::open(filename)?;
    let mut archive = ZipArchive::new(file)?;

    archive.extract("./")?;
    remove_file(filename)?;
    Ok(())
}

fn make_backup_script(path: &str) -> Result<()> {
    let bat_contents = format!(r#"@echo off
reg add "HKEY_CURRENT_USER\Environment" /v Path /t REG_EXPAND_SZ /d "{path}" /f
echo Path user environment variable restored.
pause"#);
    fs::write("HKCU.Env.Path.backup.bat", bat_contents)?;
    println!("Created a Path backup script at './HKCU.Env.Path.backup.bat'.");
    Ok(())
}

fn add_to_path(dir: &str) -> Result<()> {
    println!("Appending '{dir}' to Path...");

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (env, _) = hkcu.create_subkey("Environment")?;

    let path: String = env.get_value("Path")?;

    let mut split_path: Vec<&str> = path.split(";").filter(|x| !x.is_empty()).collect();

    if split_path.contains(&dir) {
        println!("Directory '{dir}' already exists in Path, skipping...");
        return Ok(());
    }
    println!("Directory '{dir}' does not exist in Path, appending...");

    make_backup_script(&path)?;

    split_path.push(dir);
    let path = split_path.join(";");

    env.set_value("Path", &path)?;

    Ok(())
}

fn move_files(from: &str, to: &str) -> Result<()> {
    println!("Moving files from '{from}/bin' to '{to}'...");
    let to = Path::new(to);
    create_dir_all(to)?;

    let from = Path::new(from);
    for file in fs::read_dir(from.join("bin"))? {
        let file = file?;
        let path = file.path();
        rename(
            &path,
            to.join(
                path.file_name()
                    .context("Stumbled upon a strange file.")
                    .unwrap(),
            ),
        )?;
    }

    remove_dir_all(from)?;
    Ok(())
}

fn main() -> Result<()> {
    let dest_dir = get_input(
        &format!("FFmpeg installation directory ({DEFAULT_DIR}): "),
        DEFAULT_DIR,
    )?;

    let filename = download_url(DOWNLOAD_URL)?;
    let dirname = filename.strip_suffix(".zip").context("Not a zip file.")?;

    extract_zip(&filename)?;
    add_to_path(&dest_dir)?;
    move_files(dirname, &dest_dir)?;

    println!("\n✅ Done!");
    println!("✅ FFmpeg has been successfully installed, please restart your terminal for changes to take effect.");

    Ok(())
}
