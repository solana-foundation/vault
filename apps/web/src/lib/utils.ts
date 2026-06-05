export function ellipsify(str = '', len = 4, delimiter = '..'): string {
    const strLen = str.length;
    const limit = len * 2 + delimiter.length;
    return strLen >= limit ? str.substring(0, len) + delimiter + str.substring(strLen - len, strLen) : str;
}
