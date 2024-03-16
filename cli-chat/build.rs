use bebop_tools as bebop;
use std::path::PathBuf;

fn main() {
    // download the bebop binary automatically and cache it into your target directory
    // it will automatically download the same version as the package you installed
    bebop::download_bebopc(
        PathBuf::from("target").join("bebopc"),
    );

    let config = bebop::BuildConfig{
        ..Default::default()
    };
    // build all `.bop` schemas in `schemas` dir and make a new module `generated` in `src` with all of them.
    bebop::build_schema_dir("schemas", "src/generated",&config);
}