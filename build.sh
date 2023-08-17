#!/bin/bash
# Run this file to compile. Obviously.

g++ server.cpp -o server-`uname`-`uname -m` --std=gnu++20 -g