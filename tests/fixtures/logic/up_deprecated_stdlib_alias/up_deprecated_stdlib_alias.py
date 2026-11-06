import calendar
import configparser
import glob
import pkgutil
import sys
import threading
import unittest
from threading import activeCount


def workers():
    current = threading.currentThread()
    return current.name, threading.activeCount(), activeCount()


def months(month):
    return calendar.January == month, calendar.February


def parser():
    return configparser.SafeConfigParser()


def last_error():
    return sys.last_value, sys.last_type, sys.last_traceback


def loader(name):
    return pkgutil.find_loader(name), pkgutil.get_loader(name)


def suites(case):
    return unittest.makeSuite(case), unittest.findTestCases(case), unittest.getTestCaseNames(case)


def files(pattern):
    return glob.glob0(pattern, "x"), glob.glob1(pattern, "x")


def methods(task, event):
    return task.isSet(), event.isSet(), task.setDaemon(True)


def current_names():
    return threading.current_thread, calendar.JANUARY, sys.last_exc


def reset_counters():
    sys.last_value = None
    threading.activeCount = None
