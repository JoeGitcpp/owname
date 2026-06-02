#!/bin/bash
rm -f /Users/joe/unnamed/runtime/mentor_checkpoint.json
: > /Users/joe/unnamed/logs/nursery_mentor.log
nohup python3 /Users/joe/unnamed/scripts/nursery_mentor.py --corpus > /Users/joe/unnamed/logs/nursery_mentor.log 2>&1 &
echo "mentor=$!"
