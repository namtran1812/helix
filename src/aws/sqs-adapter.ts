import type { SQSEvent } from "aws-lambda";

import type { EventEnvelope } from "../event-bus/types.js";

export interface ParsedSqsRecord {
  messageId: string;
  envelope: EventEnvelope;
}

export function parseSqsEvent(
  event: SQSEvent
): ParsedSqsRecord[] {
  return event.Records.map((record) => {
    const outer = JSON.parse(record.body) as {
      id?: string;
      source?: string;
      "detail-type"?: string;
      time?: string;
      detail?: unknown;
    };

    if (
      !outer.id ||
      !outer.source ||
      !outer["detail-type"] ||
      !outer.time ||
      !outer.detail
    ) {
      throw new Error(
        `invalid EventBridge envelope in SQS message ${record.messageId}`
      );
    }

    return {
      messageId: record.messageId,
      envelope: {
        id: outer.id,
        source: outer.source,
        detailType: outer["detail-type"] as "TraceCellEvent",
        time: outer.time,
        detail: outer.detail as EventEnvelope["detail"]
      }
    };
  });
}
