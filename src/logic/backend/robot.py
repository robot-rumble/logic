import json, sys

for inp in sys.stdin:
    robot_input = json.loads(inp)
    action = {"type": "Move", "direction": "North"}
    json.dump(action, sys.stdout)
    sys.stdout.flush()

