use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn atomic_copy(src: &Path, dest: &Path) -> std::io::Result<u64> {
    let mut reader = fs::File::open(src)?;
    let mut writer = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dest)?;
    let len = std::io::copy(&mut reader, &mut writer)?;

    // Attempt to copy permissions (best effort)
    if let Ok(meta) = fs::metadata(src) {
        let _ = writer.set_permissions(meta.permissions());
    }

    Ok(len)
}

fn main() {
    let src = Path::new("src_file");
    let dest = Path::new("dest_file");

    // Create source file with 777 permissions
    fs::write(src, "test").unwrap();
    let mut perms = fs::metadata(src).unwrap().permissions();
    perms.set_mode(0o777);
    fs::set_permissions(src, perms).unwrap();

    // Verify source permissions
    let src_mode = fs::metadata(src).unwrap().permissions().mode();
    println!("Source mode: {:o}", src_mode & 0o777);

    // Copy
    if dest.exists() {
        fs::remove_file(dest).unwrap();
    }
    atomic_copy(src, dest).unwrap();

    // Verify destination permissions
    let dest_mode = fs::metadata(dest).unwrap().permissions().mode();
    println!("Dest mode: {:o}", dest_mode & 0o777);

    if (dest_mode & 0o111) != 0 {
        println!("VULNERABLE: Destination file is executable!");
    } else {
        println!("SECURE: Destination file is not executable.");
    }
}
