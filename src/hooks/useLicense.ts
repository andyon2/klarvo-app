import { useState, useCallback, useEffect } from "react";
import type { ParsedLicenseStatus } from "../types";
import { parseLicenseStatus } from "../types";
import { getLicenseStatus, getLicenseSource, validateLicense, removeLicense, deactivateLicense } from "../tauri-commands";

export function useLicense() {
  const [licenseStatus, setLicenseStatus] = useState<ParsedLicenseStatus>({ type: "unlicensed" });
  const [licenseSource, setLicenseSource] = useState<string>("");
  const [loading, setLoading] = useState(false);

  // Load license status and source in parallel on mount.
  useEffect(() => {
    getLicenseStatus()
      .then((raw) => setLicenseStatus(parseLicenseStatus(raw)))
      .catch(console.error);
    getLicenseSource()
      .then(setLicenseSource)
      .catch(console.error);
  }, []);

  const handleValidateLicense = useCallback(async (key: string): Promise<string | null> => {
    setLoading(true);
    try {
      const raw = await validateLicense(key);
      const parsed = parseLicenseStatus(raw);
      setLicenseStatus(parsed);
      // Refresh source after validation (may have changed from hmac to lemon_squeezy).
      const src = await getLicenseSource();
      setLicenseSource(src);
      return null; // no error
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      return msg;
    } finally {
      setLoading(false);
    }
  }, []);

  const handleRemoveLicense = useCallback(async () => {
    setLoading(true);
    try {
      await removeLicense();
      // Re-fetch status from backend (may still be in trial).
      const raw = await getLicenseStatus();
      setLicenseStatus(parseLicenseStatus(raw));
      const src = await getLicenseSource();
      setLicenseSource(src);
    } catch (err) {
      console.error("remove_license failed:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  const handleDeactivateLicense = useCallback(async (): Promise<string | null> => {
    setLoading(true);
    try {
      await deactivateLicense();
      // Re-fetch status from backend (may still be in trial).
      const raw = await getLicenseStatus();
      setLicenseStatus(parseLicenseStatus(raw));
      const src = await getLicenseSource();
      setLicenseSource(src);
      return null; // no error
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error("deactivate_license failed:", err);
      return msg;
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    licenseStatus,
    licenseSource,
    licenseLoading: loading,
    validateLicense: handleValidateLicense,
    removeLicense: handleRemoveLicense,
    deactivateLicense: handleDeactivateLicense,
  };
}
