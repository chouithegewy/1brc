Areas to optimize:
- conversion from byte to float (could try implementing the #1 java result... holds temp as 3 digit integer then uses bit magic...)
- multithread result accumulator
- make look nice :)

Latest run (Ryzen 9 5590x 16core/32thread...)
```./target/release/my-1brc  159.68s user 0.67s system 2890% cpu 5.547 total```
