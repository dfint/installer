#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn main() {
  let mut res = winres::WindowsResource::new();
  res.set_icon("./assets/df.ico");
  for (id, icon) in [
    ("2", "github"),
    ("3", "transifex"),
    ("4", "trash"),
    ("5", "status-loading"),
    ("6", "status-error"),
    ("7", "status-current"),
    ("8", "status-attention"),
    ("9", "os-linux"),
    ("10", "os-windows"),
  ] {
    let path = format!("./assets/{icon}.ico");
    println!("cargo:rerun-if-changed={path}");
    res.set_icon_with_id(&path, id);
  }
  println!("cargo:rerun-if-changed=./assets/df.ico");
  println!("cargo:rerun-if-changed=build.rs");
  res.set_manifest(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
</assembly>"#,
  );
  res.compile().unwrap();
}

#[cfg(unix)]
fn main() {}
