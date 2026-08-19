import type {
  SQSBatchResponse,
  SQSEvent
} from "aws-lambda";

import { TraceProcessor } from "../correlator/processor.js";
import { DynamoTraceStore } from "./dynamo-store.js";
import { parseSqsEvent } from "./sqs-adapter.js";

const tableName = process.env.TRACE_TABLE_NAME;

if (!tableName) {
  throw new Error("TRACE_TABLE_NAME is required");
}

const store = new DynamoTraceStore(tableName);

const processor = new TraceProcessor(store);

export async function handler(
  event: SQSEvent
): Promise<SQSBatchResponse> {
  const failures: SQSBatchResponse["batchItemFailures"] = [];

  let parsed;

  try {
    parsed = parseSqsEvent(event);
  } catch (error) {
    console.error(
      JSON.stringify({
        level: "error",
        message: "failed to parse SQS batch",
        error:
          error instanceof Error
            ? error.message
            : String(error)
      })
    );

    return {
      batchItemFailures: event.Records.map(
        (record) => ({
          itemIdentifier: record.messageId
        })
      )
    };
  }

  await Promise.all(
    parsed.map(async ({ messageId, envelope }) => {
      try {
        await processor.process(envelope);

        console.log(
          JSON.stringify({
            level: "info",
            message: "processed telemetry event",
            messageId,
            eventId: envelope.id,
            traceId: envelope.detail.traceId,
            itemId: envelope.detail.itemId,
            subsystem: envelope.detail.subsystem,
            eventType: envelope.detail.eventType
          })
        );
      } catch (error) {
        failures.push({
          itemIdentifier: messageId
        });

        console.error(
          JSON.stringify({
            level: "error",
            message: "failed telemetry event",
            messageId,
            eventId: envelope.id,
            traceId: envelope.detail.traceId,
            error:
              error instanceof Error
                ? error.message
                : String(error)
          })
        );
      }
    })
  );

  return {
    batchItemFailures: failures
  };
}
