import { test as base, expect } from "@playwright/test";

export function pageUrl(namespace: string, mode?: string): string {
  const params = new URLSearchParams({ ns: namespace });
  if (mode) {
    params.set("mode", mode);
  }
  return `/?${params.toString()}`;
}

export const test = base.extend<{ namespace: string }>({
  namespace: async ({}, use, testInfo) => {
    // Per-test isolation: fullyParallel workers must not share counters.
    await use(
      `${testInfo.project.name}-p${testInfo.parallelIndex}-${testInfo.testId}`,
    );
  },
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
