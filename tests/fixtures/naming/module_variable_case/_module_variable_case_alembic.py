"""Add the users table."""

from alembic import op

revision = "1a2b3c4d5e6f"
down_revision = None
branch_labels = None
depends_on = None
table_name = "users"


def upgrade() -> None:
    op.create_table(table_name)


def downgrade() -> None:
    op.drop_table(table_name)
