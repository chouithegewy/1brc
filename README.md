Areas to optimize:
- conversion from byte to float (could try implementing the #1 java result... holds temp as 3 digit integer then uses bit magic...)
- ~~multithread result accumulator~~ (shaved off 0.172s!)
- SIMD data vectorization (allows bitmasking)
- make look nice :)

Latest run (Ryzen 9 5950X 16core/32thread...)
```./target/release/my-1brc  159.68s user 0.67s system 2890% cpu 5.547 total```

After multi-threading accumulator
```time ./target/release/my-1brc  154.41s user 0.78s system 2887% cpu 5.375 total```
