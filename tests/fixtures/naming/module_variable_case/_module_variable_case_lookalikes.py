from app_factory import create_app

revision = 3
down_revision = 2
timeout = 30
threads = 2
user = "nobody"
app = create_app()
application = None
apilevel = "2.0"
paramstyle = "qmark"
tokens = ("NUMBER",)
t_PLUS = r"\+"
