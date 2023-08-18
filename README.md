# OneDrive dump

Small utility to dump file list from OneDrive sharing link to aria2 input file with urls & checksums.

Usage:

```bash
cargo run --release -- -u <url> > list.txt
aria2c -x16 -s16 -i list.txt -c -d <output_dir>
```
