import time


def test_mocker_patch(mocker):
    mocker.patch("time.sleep")
    time.sleep(1)


def test_monkeypatch_string(monkeypatch):
    monkeypatch.setattr("time.sleep", lambda seconds: None)
    time.sleep(1)


def test_monkeypatch_module(monkeypatch):
    monkeypatch.setattr(time, "sleep", lambda seconds: None)
    time.sleep(1)
