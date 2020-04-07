import json, sys

while True:
    print("asdf", input())
    robot_input = json.load(sys.stdin)
    action = {"type": "Move", "direction": "North"}
    json.dump(action, sys.stdout)

