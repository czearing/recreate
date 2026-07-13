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
