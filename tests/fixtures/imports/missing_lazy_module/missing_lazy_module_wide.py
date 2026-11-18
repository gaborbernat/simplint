__lazy_modules__ = ["argparse", "concurrent.futures", "email.mime.multipart", "http.server", "xml.etree.ElementTree"]

import argparse
import concurrent.futures
import email.mime.multipart
import http.server
import json
import xml.etree.ElementTree


def parse() -> object:
    return argparse, concurrent.futures, email.mime.multipart, http.server, json, xml.etree.ElementTree
