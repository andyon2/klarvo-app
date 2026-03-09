import { useState, useCallback, useEffect } from "react";
import type { ParsedLicenseStatus } from "../types";
import { parseLicenseStatus } from "../types";
import { getLicenseStatus, validateLicense, removeLicense } from "../tauri-commands";

export function useLicense() {
  const [licenseStatus, setLicenseStatus] = useState<ParsedLicenseStatus>({ type: "unlicensed" });
  const [loading, setLoading] = useState(false);

  // Load license status on mount.
  useEffect(() => {
    getLicenseStatus()
      .then((raw) => setLicenseStatus(parseLicenseStatus(raw)))
      .catch(console.error);
  }, []);

  const handleValidateLicense = useCallback(async (key: string): Promise<string | null> => {
    setLoading(true);
    try {
      const raw = await validateLicense(key);
      const parsed = parseLicenseStatus(raw);
      setLicenseStatus(parsed);
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
      setLicenseStatus({ type: "unlicensed" });
    } catch (err) {
      console.error("remove_license failed:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    licenseStatus,
    licenseLoading: loading,
    validateLicense: handleValidateLicense,
    removeLicense: handleRemoveLicense,
  };
}
