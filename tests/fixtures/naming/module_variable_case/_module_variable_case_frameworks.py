from flask import Flask
from pyramid.config import Configurator

bind = "unix:/run/app.sock"
worker_class = "gthread"
threads = 4
app = Flask(__name__)
configurator = Configurator()
application: object = configurator.make_wsgi_app()
