# ESP Now Connection

This is just a small ESP32 program which establishes a connection with another device
running the same program. Both devices ping each other and if a ping is not recieved
in a given timeout, the connection is removed.

## Why?

I wanted to...
1. See if there was a way to calculate or estimate the distance between both ESP32s.
2. Better understand how the ESP Now protocol and embedded rust work.

