export function interactionMatchPriority(candidate, match) {
  if (!match) return 0;
  const label = `${candidate.text || ''} ${candidate.placeholder || ''}`
    .trim()
    .toLowerCase();
  if (label === match) return 2;
  return label.includes(match) ? 1 : 0;
}

export function candidateUsesTextEntry(candidate) {
  const textInput =
    candidate.inputType === 'text' ||
    candidate.tag === 'TEXTAREA';
  const popupInput =
    candidate.role === 'combobox' ||
    /(?:listbox|tree|menu|dialog)/i.test(candidate.ariaHaspopup || '');
  return textInput && !popupInput;
}

export function selectInteractionIdentity(expected, identities) {
  const normalize = (value) => String(value || '').replace(/\s+/g, ' ').trim();
  const wanted = normalize(expected);
  const labels = identities.map(normalize);
  const exact = labels.findIndex((label) => label === wanted);
  if (exact >= 0) return exact;
  const prefix = wanted.slice(0, 20);
  const partial = labels
    .map((label, index) => label.startsWith(prefix) ? index : -1)
    .filter((index) => index >= 0);
  return partial.length === 1 ? partial[0] : -1;
}

export const selectInteractionIdentityRuntimeSource =
  `(${selectInteractionIdentity.toString()})`;

export function interactionCandidatePriority(candidate) {
  if (candidate.inputType === 'submit' || candidate.buttonType === 'submit') {
    return candidate.formRequiredCount
      ? 110 + Math.min(20, Math.log10(Math.max(1, candidate.formArea)))
      : 60;
  }
  if (candidate.ariaHaspopup === 'true' ||
      /(?:menu|listbox|tree|dialog)/i.test(candidate.ariaHaspopup || '')) {
    return candidate.topBar ? 85 : 105;
  }
  if (candidate.popoverTarget) return 105;
  if (candidate.testId === 'notebook-card') return 104;
  if ((candidate.inputType === 'text' || candidate.tag === 'TEXTAREA') &&
      candidate.hasFormSubmit) return 20;
  if (candidate.inputType === 'text' || candidate.tag === 'TEXTAREA') return 95;
  if (candidate.href) return 100;
  if (candidate.role === 'link' || candidate.tag === 'A') return 80;
  if (candidate.inputType === 'checkbox' || candidate.inputType === 'radio') return 70;
  if (candidate.tag === 'BUTTON') return 60;
  if (candidate.topBar) return 1;
  if (candidate.role === 'button') return 50;
  return 10;
}

export function interactionSettleTimeout(candidate) {
  return candidate.testId === 'notebook-card' ? 8000 : 3000;
}

export function interactionStateSettleDelay(candidate, explicitlyMatched = false) {
  if (candidate.testId === 'notebook-card') return 5000;
  return explicitlyMatched ? 1200 : 600;
}
