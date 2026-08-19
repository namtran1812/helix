import {
  DynamoDBClient,
  GetItemCommand,
  PutItemCommand,
  QueryCommand
} from "@aws-sdk/client-dynamodb";

import type {
  ItemTrace,
  TraceEvent
} from "../events/types.js";

export class DynamoTraceStore {
  constructor(
    private readonly tableName: string,
    private readonly client = new DynamoDBClient({})
  ) {}

  async putEvent(event: TraceEvent): Promise<boolean> {
    try {
      await this.client.send(
        new PutItemCommand({
          TableName: this.tableName,
          Item: {
            pk: { S: `TRACE#${event.traceId}` },
            sk: { S: `EVENT#${event.eventId}` },
            type: { S: "EVENT" },
            payload: {
              S: JSON.stringify(event)
            }
          },
          ConditionExpression:
            "attribute_not_exists(pk) AND attribute_not_exists(sk)"
        })
      );

      return true;
    } catch (error) {
      if (
        error instanceof Error &&
        error.name === "ConditionalCheckFailedException"
      ) {
        return false;
      }

      throw error;
    }
  }

  async getEvents(traceId: string): Promise<TraceEvent[]> {
    const response = await this.client.send(
      new QueryCommand({
        TableName: this.tableName,
        KeyConditionExpression:
          "pk = :pk AND begins_with(sk, :prefix)",
        ExpressionAttributeValues: {
          ":pk": { S: `TRACE#${traceId}` },
          ":prefix": { S: "EVENT#" }
        }
      })
    );

    return (response.Items ?? []).map((item) => {
      const payload = item.payload?.S;

      if (!payload) {
        throw new Error("event record missing payload");
      }

      return JSON.parse(payload) as TraceEvent;
    });
  }

  async putTrace(trace: ItemTrace): Promise<void> {
    await this.client.send(
      new PutItemCommand({
        TableName: this.tableName,
        Item: {
          pk: { S: `TRACE#${trace.traceId}` },
          sk: { S: "MATERIALIZED" },
          type: { S: "TRACE" },
          payload: {
            S: JSON.stringify(trace)
          }
        }
      })
    );
  }

  async getTrace(
    traceId: string
  ): Promise<ItemTrace | undefined> {
    const response = await this.client.send(
      new GetItemCommand({
        TableName: this.tableName,
        Key: {
          pk: { S: `TRACE#${traceId}` },
          sk: { S: "MATERIALIZED" }
        }
      })
    );

    const payload = response.Item?.payload?.S;

    if (!payload) {
      return undefined;
    }

    return JSON.parse(payload) as ItemTrace;
  }
}
