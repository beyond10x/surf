#!/bin/sh
# Adversary stand-in model: one surf-agent call whose one argument word holds a line feed.

surf-agent 'look
task article pass 9 calls' > /dev/null 2>&1

echo 'ANSWER: unknown'
