#!/usr/bin/env python3
"""Compatibility entry point for the authenticated network-worker integration suite.
The former SSH/star-mesh fixture has been replaced by real outbound workers.
"""
import runpy
from pathlib import Path
runpy.run_path(str(Path(__file__).with_name('verify-cloud.py')),run_name='__main__')
