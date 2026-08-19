import assert from "node:assert/strict";
import test from "node:test";

import type { SQSEvent } from "aws-lambda";

import { createEnvelope } from "../../src/event-bus/envelope.js";
import { runSimulation } from "../../src/simulators/run.js";
import { parseSqsEvent } from "../../src/aws/sqs-adapter.js";

const context = {
  itemId: "ITEM-000001",
  traceId: "TRACE-000001",
  robotId: "ROBOT-017",
  workcellId: "CELL-04",
  containerId: "BIN-0291",
  startMs: Date.UTC(2026, 7, 19, 16, 0, 0)
};

test("parses EventBridge payload delivered through SQS", () => {
  const event = runSimulation(context)[0];

  const envelope = createEnvelope(event);

  const sqsEvent: SQSEvent = {
    Records: [
      {
        messageId: "message-1",
        receiptHandle: "receipt",
        body: JSON.stringify({
          id: envelope.id,
          source: envelope.source,
          "detail-type": envelope.detailType,
          time: envelope.time,
          detail: envelope.detail
        }),
        attributes: {
          ApproximateReceiveCount: "1",
          SentTimestamp: "0",
          SenderId: "test",
          ApproximateFirstReceiveTimestamp: "0"
        },
        messageAttributes: {},
        md5OfBody: "",
        eventSource: "aws:sqs",
        eventSourceARN:
          "arn:aws:sqs:us-east-1:123456789012:test",
        awsRegion: "us-east-1"
      }
    ]
  };

  const parsed = parseSqsEvent(sqsEvent);

  assert.equal(parsed.length, 1);
  assert.equal(parsed[0].messageId, "message-1");
  assert.equal(parsed[0].envelope.id, event.eventId);
  assert.equal(
    parsed[0].envelope.detail.traceId,
    context.traceId
  );
});
