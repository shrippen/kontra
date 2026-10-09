# Import samples

Tests in `crates/content` build their fixtures in code (`crates/content/tests/common/`), so no
copyrighted files live in git.

To check the importers against real files, put them into `testdata/import/local/`
(gitignored: `/testdata/import/local/`) and run:

```sh
cargo test -p kontra-content --test local_files -- --ignored --nocapture
```

`imports_local_files` imports every file there, builds an arrangement for every track and
lists failures; files without notes are reported as `empty`. `local_files_survive_corruption`
damages each file and checks that the importers return errors instead of panicking.
