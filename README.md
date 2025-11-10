TODO:
- fix station name buf (station name min_max_buf all 0s) -- change to &str instead of [u8; 100]
- aggregate avg per weather station (not total) - "{Abha=-23.0/18.0/59.2, Abidjan=-16.2/26.0/67.3, Abéché=-10.0/29.4/69.0, Accra=-10.1/26.4/66.4, Addis Ababa=-23.7/16.0/67.0, Adelaide=-27.8/17.3/58.5, ...}"
- accumulate all StationResults, sort

```
$ time ./target/release/my-1brc
FINAL RESULT: max: 90; min: 0; mean: 17.358883; sum: 16816446000
./target/release/my-1brc  49.85s user 0.70s system 2702% cpu 1.870 total
```
