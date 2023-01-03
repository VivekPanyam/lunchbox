// Copyright 2023 Vivek Panyam
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

/// Replace inline code blocks in the README with links when generating docs
fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=README.md");

    let out_dir = std::env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("README_processed.md");
    let mut f = BufWriter::new(File::create(&dest_path)?);

    let readme = std::fs::read_to_string("README.md")?;

    // Not ideal, but probably fine for now
    let processed = readme
        .replace(
            "`ReadableFile`",
            "[`ReadableFile`](crate::types::ReadableFile)",
        )
        .replace(
            "`WritableFile`",
            "[`WritableFile`](crate::types::WritableFile)",
        )
        .replace("`LocalFS`", "[`LocalFS`]")
        .replace("`ReadableFileSystem`", "[`ReadableFileSystem`]")
        .replace("`WritableFileSystem`", "[`WritableFileSystem`]")
        .replace("`tokio::fs`", "[`tokio::fs`]")
        .replace("`std::path::Path`", "[`std::path::Path`]")
        .replace(
            "`lunchbox::path::Path`",
            "[`lunchbox::path::Path`](crate::path::Path)",
        )
        .replace(
            "`lunchbox::path::PathBuf`",
            "[`lunchbox::path::PathBuf`](crate::path::PathBuf)",
        )
        .replace(
            "`LunchboxPathUtils`",
            "[`LunchboxPathUtils`](crate::path::LunchboxPathUtils)",
        );

    write!(f, "{}", processed)?;

    Ok(())
}
