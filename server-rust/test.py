import sqlite3 as sql

cursor = sql.connect("default.db")
teamreader = cursor.execute("SELECT * FROM teams_records")
teams = []

while True:
    tdata = teamreader.fetchone()
    if tdata == None:
        break
    teams.append((tdata[0], tdata[1] / (tdata[1] + tdata[2]),))

print(teams)
