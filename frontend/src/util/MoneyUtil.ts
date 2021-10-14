export function formatCentsAsMoney(cents: number): string {
  const string = cents.toFixed(0).padStart(3, "0");
  return (
    string.substring(0, string.length - 2) +
    "," +
    string.substring(string.length - 2)
  );
}
