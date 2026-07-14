import { describe, expect, it, vi } from "vitest";
import {
  clearSessionCache,
  createApiClient,
  getSessionCache,
  setSessionUser,
  validatePassword,
  validateReservationForm,
} from "./client";

describe("validatePassword", () => {
  it("rejects short passwords", () => {
    expect(validatePassword("short")).toBeTruthy();
  });

  it("accepts 12+ chars", () => {
    expect(validatePassword("twelvechars!")).toBeNull();
  });
});

describe("validateReservationForm", () => {
  it("requires equipment and ordered range", () => {
    expect(
      validateReservationForm({
        equipmentId: "",
        startsAt: "2026-07-14T10:00:00Z",
        endsAt: "2026-07-14T12:00:00Z",
      }),
    ).toBeTruthy();
    expect(
      validateReservationForm({
        equipmentId: "eq-1",
        startsAt: "2026-07-14T12:00:00Z",
        endsAt: "2026-07-14T10:00:00Z",
      }),
    ).toBeTruthy();
    expect(
      validateReservationForm({
        equipmentId: "eq-1",
        startsAt: "2026-07-14T10:00:00Z",
        endsAt: "2026-07-14T12:00:00Z",
      }),
    ).toBeNull();
  });
});

describe("createApiClient auth refresh", () => {
  it("retries once after successful refresh on 401", async () => {
    clearSessionCache();
    const fetchImpl = vi
      .fn()
      .mockResolvedValueOnce(new Response("unauthorized", { status: 401 }))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            id: "1",
            email: "u@example.com",
            role: "user",
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ id: "1", email: "u@example.com", role: "user" }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );

    const api = createApiClient({ fetchImpl });
    const me = await api.me();
    expect(me.email).toBe("u@example.com");
    expect(fetchImpl).toHaveBeenCalledTimes(3);
    expect(getSessionCache().user?.email).toBe("u@example.com");
  });

  it("clears session cache when refresh fails", async () => {
    setSessionUser({ id: "1", email: "u@example.com", role: "user" });
    const onUnauthorized = vi.fn();
    const fetchImpl = vi
      .fn()
      .mockResolvedValueOnce(new Response("unauthorized", { status: 401 }))
      .mockResolvedValueOnce(new Response("unauthorized", { status: 401 }));

    const api = createApiClient({ fetchImpl, onUnauthorized });
    await expect(api.me()).rejects.toThrow("unauthorized");
    expect(getSessionCache().user).toBeNull();
    expect(onUnauthorized).toHaveBeenCalled();
  });
});
