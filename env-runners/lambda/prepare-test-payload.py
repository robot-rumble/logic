import argparse
import json

parser = argparse.ArgumentParser(description='Prepare file for aws test')
parser.add_argument('robot_file1', type=argparse.FileType('r'))
parser.add_argument('robot_file2', type=argparse.FileType('r'))
args = parser.parse_args()

def prepare(code):
    return code

code1 = prepare(args.robot_file1.read())
code2 = prepare(args.robot_file2.read())

request = json.loads("""
  {
    "Records": [
        {
            "messageId": "19dd0b57-b21e-4ac1-bd88-01bbb068cb78",
          "receiptHandle": "MessageReceiptHandle",
          "body": "",
        "attributes": {
        "ApproximateReceiveCount": "1",
            "SentTimestamp": "1523232000000",
            "SenderId": "123456789012",
            "ApproximateFirstReceiveTimestamp": "1523232000001"
          },
              "messageAttributes": {},
          "md5OfBody": "7b270e59b47ff90a553787216d55d91d",
          "eventSource": "aws:sqs",
          "eventSourceARN": "arn:aws:sqs:us-east-1:123456789012:MyQueue",
          "awsRegion": "us-east-1"
        }
      ]
    }
""")

body = json.loads("""
    {"r1_id":0, "pr1_id":0, "r1_lang": "Python", "r1_code":"","r2_id":0,"pr2_id":0, "r2_lang": "Python", "r2_code":""}
""")

body["r1_code"] = code1
body["r2_code"] = code2

request['Records'][0]['body'] = json.dumps(body)
print(json.dumps(request))
