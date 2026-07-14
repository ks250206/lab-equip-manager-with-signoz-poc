import { OTLPTraceExporter } from "@opentelemetry/exporter-trace-otlp-http";
import { registerInstrumentations } from "@opentelemetry/instrumentation";
import { FetchInstrumentation } from "@opentelemetry/instrumentation-fetch";
import { resourceFromAttributes } from "@opentelemetry/resources";
import { BatchSpanProcessor } from "@opentelemetry/sdk-trace-base";
import { WebTracerProvider } from "@opentelemetry/sdk-trace-web";
import { ZoneContextManager } from "@opentelemetry/context-zone";
import { ATTR_SERVICE_NAME } from "@opentelemetry/semantic-conventions";

export function initOtel(): void {
  const endpoint = import.meta.env.VITE_OTEL_ENDPOINT ?? "http://localhost:14318";

  const provider = new WebTracerProvider({
    resource: resourceFromAttributes({
      [ATTR_SERVICE_NAME]: "equipment-reservation-web",
    }),
    spanProcessors: [
      new BatchSpanProcessor(
        new OTLPTraceExporter({
          url: `${endpoint}/v1/traces`,
        }),
      ),
    ],
  });

  provider.register({
    contextManager: new ZoneContextManager(),
  });

  registerInstrumentations({
    instrumentations: [
      new FetchInstrumentation({
        propagateTraceHeaderCorsUrls: [/.*/],
        clearTimingResources: true,
      }),
    ],
  });
}
