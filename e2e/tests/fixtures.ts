import { test as base, expect } from "@playwright/test";

export function pageUrl(namespace: string, mode?: string): string {
  const params = new URLSearchParams({ ns: namespace });
  if (mode) {
    params.set("mode", mode);
  }
  return `/?${params.toString()}`;
}

export const test = base.extend<object, { namespace: string }>({
  namespace: [
    async ({}, use, workerInfo) => {
      await use(`${workerInfo.project.name}-w${workerInfo.workerIndex}`);
    },
    { scope: "worker" },
  ],
});

export { expect };

async function resetCounter(
  request: import("@playwright/test").APIRequestContext,
  namespace: string,
) {
  await request.post("/api/counter/reset", { data: { namespace } });
}

test.beforeEach(async ({ request, namespace }) => {
  await resetCounter(request, namespace);
});

export { resetCounter };
