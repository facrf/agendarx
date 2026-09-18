export const riskLabels = {
  NAO_CLASSIFICADO: "Não classificado",
  SEM_RISCO: "Sem risco cadastrado",
  MANIPULATIVO: "Influência interpessoal",
  PATOLOGICO: "Risco observado",
  MISTO: "Múltiplos fatores",
} as const;

export function reviewDate(value: string | null | undefined) {
  return value ? value.split("-").reverse().join("/") : "Não informada";
}

export function todayReviewDate() {
  const today = new Date();
  return `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
}
