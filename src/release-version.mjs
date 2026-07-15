const SEMVER_PATTERN = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?$/;

export function getStableVersion(version) {
  const match = SEMVER_PATTERN.exec(version);
  if (!match) {
    throw new Error(`Invalid semantic version: ${version}`);
  }
  return `${match[1]}.${match[2]}.${match[3]}`;
}

export function getBetaVersion(version, runNumber, runAttempt, commitSha) {
  const stableVersion = getStableVersion(version);
  if (!/^[1-9]\d*$/.test(String(runNumber))) {
    throw new Error(`Invalid run number: ${runNumber}`);
  }
  if (!/^[1-9]\d*$/.test(String(runAttempt))) {
    throw new Error(`Invalid run attempt: ${runAttempt}`);
  }
  if (!/^[0-9a-f]{7,40}$/i.test(commitSha)) {
    throw new Error(`Invalid commit SHA: ${commitSha}`);
  }
  return `${stableVersion}-beta.${runNumber}.${runAttempt}.sha-${commitSha.slice(0, 7).toLowerCase()}`;
}
