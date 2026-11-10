#!/bin/bash
set -e

./prepare-source.sh
debuild -S -sa
