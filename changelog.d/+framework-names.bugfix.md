`module-variable-case` (SMP102) no longer asks to rename module variables a framework reads by name: the revision
identifiers of an Alembic migration, the settings of a gunicorn configuration file, and an `app` or `application` that
builds a WSGI or ASGI application with Flask, FastAPI, Starlette, Django, Quart, Sanic, aiohttp, Litestar, Falcon,
Bottle or Pyramid.
