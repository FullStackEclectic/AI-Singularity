/**
 * 隐私模式工具 — 一键脱敏敏感信息
 */

const PRIVACY_KEY = 'ai_singularity.privacy_mode';

export function isPrivacyMode(): boolean {
  try {
    return localStorage.getItem(PRIVACY_KEY) === 'true';
  } catch {
    return false;
  }
}

export function setPrivacyMode(enabled: boolean): void {
  try {
    localStorage.setItem(PRIVACY_KEY, enabled ? 'true' : 'false');
  } catch {
    // ignore
  }
}

/**
 * 脱敏邮箱：a***@***.com
 * 例如：foo@bar.com → f**@b**.com
 */
export function maskEmail(email: string): string {
  if (!email || !email.includes('@')) return '****';
  const [local, domain] = email.split('@');
  const maskedLocal = local.length > 1 ? local[0] + '***' : '***';
  const domainParts = domain.split('.');
  const maskedDomain =
    domainParts[0].length > 1
      ? domainParts[0][0] + '***'
      : '***';
  const tld = domainParts.slice(1).join('.');
  return `${maskedLocal}@${maskedDomain}.${tld}`;
}

/**
 * 脱敏 Token：保留前4后4位
 * 例如：sk-1234567890abcdef → sk-1****cdef
 */
export function maskToken(token: string): string {
  if (!token) return '****';
  if (token.length <= 8) return '****';
  return token.slice(0, 4) + '****' + token.slice(-4);
}

/**
 * 根据隐私模式条件脱敏
 */
export function maybeEmail(email: string, privacy: boolean): string {
  return privacy ? maskEmail(email) : email;
}

export function maybeToken(token: string, privacy: boolean): string {
  return privacy ? maskToken(token) : token;
}
