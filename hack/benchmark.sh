#!/bin/sh
time parallel -j 100 dig @127.0.0.1 -p 8080 {} < domains.txt