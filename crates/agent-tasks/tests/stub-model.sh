#!/bin/sh
# The stand-in model of model_driver::a_stub_model_is_counted: it calls surf-agent once, as
# `surf-agent look` (no session is set, so it exits 2 and nothing starts), and answers by the
# question in its one argument, the prompt.

surf-agent look > /dev/null 2>&1

case $1 in
  *'In which year was the bridge opened?'*) echo 'ANSWER: 1932' ;;
  *'What does one kilo of pears cost?'*) echo 'ANSWER: 2.41' ;;
  *) echo 'ANSWER: unknown' ;;
esac
