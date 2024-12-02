# AoC timeline

To display a timeline and standing for multiple leaderboards in Advent of Code

Prereqs:
1. cargo install ssclient
2. ssclient create --export-key .secrets.key
3.
    1. ssclient -k .secrets.key set session
    2. Get session cookie from browser and paste in ssclient prompt
4. Change YEAR and LEADERBOARDS in src/main.rs

Next year, repeat step 3 and 4.
