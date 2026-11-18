__lazy_modules__ = ["json", "csv"]

lazy import json
lazy import os
lazy from sys import argv, path
lazy \
    import abc
lazy import sys, zipfile
lazy from csv import reader
import csv
import pathlib
lazy from pathlib import Path


def use() -> object:
    return argv, path, json, os, abc, sys, zipfile, reader, csv, pathlib, Path
