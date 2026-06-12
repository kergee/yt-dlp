import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import packageInfo from "../package.json";
import { thumbnailUrlCandidates } from "./thumbnail";

type ToolStatus = {
  name: string;
  relative_path: string;
  full_path: string;
  availability: "available" | "missing" | "cannot_execute";
  version?: string;
  error?: string;
};

type VideoFormatOption = {
  label: string;
  format_selector: string;
  height?: number;
  extension: string;
  is_best: boolean;
};

type VideoMetadata = {
  title: string;
  id?: string;
  webpage_url: string;
  thumbnail_url?: string;
  thumbnail_urls?: string[];
  duration_seconds?: number;
  description?: string;
  format_options: VideoFormatOption[];
};

type DownloadProgress = {
  percent?: number;
  speed?: string;
  eta?: string;
  status: "downloading" | "processing" | "finished" | "error";
};

const APP_VERSION = packageInfo.version;
type AppState = {
  download_directory: string;
  tools_root: string;
  yt_dlp_path: string;
  ffmpeg_path: string;
  ffprobe_path: string;
  deno_path: string;
  cookies_file: string | null;
  cookies_status: "none" | "valid" | "warning";
};

type Language = "en" | "zh";
type TranslationKey = string;

const translations: Record<Language, Record<string, string>> = {
  en: {
    "app.title": "Universal Video Downloader",
    "app.eyebrow": "Desktop downloader for yt-dlp",
    "app.heading": "Paste, choose, download.",
    "language.label": "Language",
    "action.settings": "Settings",
    "url.label": "Video URL",
    "url.placeholder": "https://www.youtube.com/watch?v=...",
    "action.parse": "Parse",
    "cookies.label": "Cookie file",
    "action.webLogin": "Web Login",
    "action.chooseCookies": "Choose Cookie file",
    "action.clearCookies": "Clear",
    "preview.thumbnailAlt": "Video thumbnail",
    "preview.emptyImage": "Preview",
    "preview.label": "Preview",
    "download.quality": "Quality",
    "action.download": "Download",
    "action.cancel": "Cancel",
    "action.openFolder": "Open folder",
    "settings.kicker": "Preferences",
    "settings.title": "Settings",
    "action.close": "Close",
    "settings.outputFolder": "Output folder",
    "action.browse": "Browse",
    "action.save": "Save",
    "action.reset": "Reset",
    "settings.toolchain": "Toolchain",
    "settings.toolchainHint": "Please manually download the required tools to the path below.",
    "action.refresh": "Refresh",
    "settings.version": "Version",
    "preview.emptyStart": "Enter a URL to preview...",
    "preview.emptyChanged": "URL changed, click Parse to analyze",
    "notice.checkingTools": "Checking tools...",
    "progress.idle": "Idle",
    "notice.cookiesWarning": "Cookies might have expired. Please login again if downloads fail.",
    "notice.toolchainReady": "Toolchain is ready.",
    "notice.toolsMissing": "Toolchain is missing or incomplete.",
    "settings.toolCheckFailed": "Failed to check tools: {error}",
    "notice.wechatIntercept": "WeChat video detected, sync starting...",
    "progress.parsing": "Parsing metadata...",
    "preview.readingMetadata": "Reading metadata...",
    "progress.metadataReady": "Metadata parsed successfully.",
    "notice.metadataParsed": "Metadata parsed successfully.",
    "preview.parseFailed": "Failed to parse video. Make sure the URL is valid.",
    "progress.metadataFailed": "Failed to parse metadata.",
    "progress.startingDownload": "Starting download ({quality})...",
    "progress.savedTo": "Saved to {path}",
    "progress.completedOpenFolder": "Download completed, click Open folder",
    "notice.downloadCompleted": "Download completed successfully.",
    "progress.downloadCancelled": "Download cancelled.",
    "notice.downloadCancelled": "Download cancelled.",
    "progress.downloadFailed": "Download failed.",
    "progress.cancelling": "Cancelling download...",
    "settings.chooseFolder": "Choose Output Folder",
    "notice.folderUpdated": "Download folder updated.",
    "notice.folderReset": "Download folder reset to default.",
    "cookies.chooseFile": "Choose Cookies File",
    "preview.cookiesChanged": "Cookies changed, please re-analyze",
    "notice.cookiesUpdated": "Cookie file updated.",
    "notice.cookiesSynced": "Cookies synchronized from webview.",
    "notice.cookiesCleared": "Cookie file cleared.",
    "preview.noVideo": "No video parsed",
    "progress.eta": "ETA:",
    "cookies.none": "No cookies",
    "notice.linksCopied": "Toolchain download links copied to clipboard!",
    "settings.chooseTool": "Select {name} binary",
    "notice.toolchainSaved": "Toolchain paths saved successfully."
  },
  zh: {
    "app.title": "全能视频下载器",
    "app.eyebrow": "yt-dlp 桌面端",
    "app.heading": "解析、选择、下载。",
    "language.label": "语言",
    "action.settings": "设置",
    "url.label": "视频链接",
    "url.placeholder": "请输入视频链接...",
    "action.parse": "解析链接",
    "cookies.label": "Cookie 文件",
    "action.webLogin": "网页登录",
    "action.chooseCookies": "选择 Cookie 文件",
    "action.clearCookies": "清除",
    "preview.thumbnailAlt": "视频缩略图",
    "preview.emptyImage": "预览",
    "preview.label": "预览",
    "download.quality": "画质",
    "action.download": "下载视频",
    "action.cancel": "取消下载",
    "action.openFolder": "打开文件夹",
    "settings.kicker": "偏好设置",
    "settings.title": "设置",
    "action.close": "关闭",
    "settings.outputFolder": "下载保存目录",
    "action.browse": "浏览",
    "action.save": "保存",
    "action.reset": "重置",
    "settings.toolchain": "工具链",
    "settings.toolchainHint": "请手动下载所需的工具至下方路径。",
    "action.refresh": "刷新",
    "settings.version": "版本",
    "preview.emptyStart": "输入视频链接后点击解析",
    "preview.emptyChanged": "链接已更改，点击解析分析",
    "notice.checkingTools": "正在检查工具链...",
    "progress.idle": "空闲",
    "notice.cookiesWarning": "Cookie 可能已过期，如果下载失败请重新登录。",
    "notice.toolchainReady": "工具链就绪。",
    "notice.toolsMissing": "工具链缺失或不完整。",
    "settings.toolCheckFailed": "检查工具链失败: {error}",
    "notice.wechatIntercept": "检测到微信视频，开始同步...",
    "progress.parsing": "正在解析视频信息...",
    "preview.readingMetadata": "正在读取视频元数据...",
    "progress.metadataReady": "元数据解析成功。",
    "notice.metadataParsed": "元数据解析成功。",
    "preview.parseFailed": "解析视频失败。请确保链接有效。",
    "progress.metadataFailed": "解析元数据失败。",
    "progress.startingDownload": "开始下载 ({quality})...",
    "progress.savedTo": "已保存至 {path}",
    "progress.completedOpenFolder": "下载完成，点击打开文件夹",
    "notice.downloadCompleted": "下载完成。",
    "progress.downloadCancelled": "下载被取消。",
    "notice.downloadCancelled": "下载被取消。",
    "progress.downloadFailed": "下载失败。",
    "progress.cancelling": "正在取消下载...",
    "settings.chooseFolder": "选择下载保存目录",
    "notice.folderUpdated": "下载目录已更新。",
    "notice.folderReset": "下载目录已重置为默认。",
    "cookies.chooseFile": "选择 Cookie 文件",
    "preview.cookiesChanged": "Cookie 已更改，请重新解析",
    "notice.cookiesUpdated": "Cookie 文件已更新。",
    "notice.cookiesSynced": "Cookie 已从网页同步。",
    "notice.cookiesCleared": "Cookie 已清除。",
    "preview.noVideo": "未解析视频",
    "progress.eta": "剩余时间:",
    "cookies.none": "未设置 Cookie",
    "notice.linksCopied": "工具下载地址已复制到剪贴板！",
    "settings.chooseTool": "选择 {name} 可执行文件",
    "notice.toolchainSaved": "工具链路径保存成功。"
  }
};

type NoticeTone = "info" | "success" | "warning" | "error";
type GithubAccessMode = "direct" | "proxy";

function resolveInitialGithubAccessMode(): GithubAccessMode {
  const stored = localStorage.getItem("yt-dlp-tauri-github-access");
  if (stored === "direct" || stored === "proxy") {
    return stored;
  }
  return "direct";
}

function resolveInitialLanguage(): Language {
  const stored = localStorage.getItem("yt-dlp-tauri-language");
  if (stored === "en" || stored === "zh") {
    return stored;
  }
  return navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
}


const state = {
  language: resolveInitialLanguage(),
  metadata: null as VideoMetadata | null,
  busy: false,
  parsedUrl: "",
  parsedFormat: "",
  githubAccessMode: resolveInitialGithubAccessMode(),
  cookiesFile: null as string | null,
  cookiesStatus: "none" as "none" | "valid" | "warning",
  toolsReady: false,
  thumbnailCandidateIndex: 0,
  thumbnailCandidates: [] as string[],
  activeOperation: null as "metadata" | "download" | "tools" | null,
  cancelRequested: false,
  selectedFormat: null as VideoFormatOption | null,
  lastUrl: "",
  noticeKey: null as string | null,
  noticeArgs: {} as Record<string, string | number>,
  noticeRaw: null as string | null,
  noticeTone: "warning" as NoticeTone,
  progressKey: "progress.idle" as string | null,
  progressArgs: {} as Record<string, string | number>,
  progressRaw: null as string | null,
  currentDownloadProgress: null as DownloadProgress | null,
};

const elements = {
  url: must<HTMLInputElement>("#url"),
  parse: must<HTMLButtonElement>("#parse"),
  cookiesFile: must<HTMLElement>("#cookies-file"),
  webLogin: must<HTMLButtonElement>("#web-login"),
  chooseCookies: must<HTMLButtonElement>("#choose-cookies"),
  clearCookies: must<HTMLButtonElement>("#clear-cookies"),
  thumbnail: must<HTMLImageElement>("#thumbnail"),
  thumbnailEmpty: must<HTMLElement>("#thumbnail-empty"),
  title: must<HTMLElement>("#video-title"),
  details: must<HTMLElement>("#video-details"),
  description: must<HTMLElement>("#video-description"),
  quality: must<HTMLSelectElement>("#quality"),
  download: must<HTMLButtonElement>("#download"),
  cancel: must<HTMLButtonElement>("#cancel"),
  openFolder: must<HTMLButtonElement>("#open-folder"),
  progress: must<HTMLProgressElement>("#progress"),
  progressText: must<HTMLElement>("#progress-text"),
  settingsBackdrop: must<HTMLElement>("#settings-backdrop"),
  settingsDrawer: must<HTMLElement>("#settings-drawer"),
  settingsClose: must<HTMLButtonElement>("#settings-close"),
  settingsToggle: must<HTMLButtonElement>("#settings-toggle"),
  languageEn: must<HTMLButtonElement>("#language-en"),
  languageZh: must<HTMLButtonElement>("#language-zh"),
  folderText: must<HTMLElement>("#folder-text"),
  folderInput: must<HTMLInputElement>("#folder-input"),
  browseFolder: must<HTMLButtonElement>("#browse-folder"),
  saveFolder: must<HTMLButtonElement>("#save-folder"),
  resetFolder: must<HTMLButtonElement>("#reset-folder"),
  toolsInfoBtn: must<HTMLButtonElement>("#tools-info-btn"),
  ytDlpInput: must<HTMLInputElement>("#yt-dlp-input"),
  ffmpegInput: must<HTMLInputElement>("#ffmpeg-input"),
  ffprobeInput: must<HTMLInputElement>("#ffprobe-input"),
  denoInput: must<HTMLInputElement>("#deno-input"),
  browseYtDlp: must<HTMLButtonElement>("#browse-yt-dlp"),
  browseFfmpeg: must<HTMLButtonElement>("#browse-ffmpeg"),
  browseFfprobe: must<HTMLButtonElement>("#browse-ffprobe"),
  browseDeno: must<HTMLButtonElement>("#browse-deno"),
  saveTools: must<HTMLButtonElement>("#save-tools"),
  notice: must<HTMLElement>("#notice"),
  appVersion: must<HTMLElement>("#app-version"),
  progressContainer: must<HTMLElement>("#progress"),

};

function must<T extends HTMLElement = HTMLElement>(selector: string): T {
  const element = document.querySelector<T>(selector);
  if (!element) {
    throw new Error(`Element not found: ${selector}`);
  }
  return element;
}





function t(key: TranslationKey, values: Record<string, string | number> = {}) {
  let text: string = translations[state.language][key] || translations.en[key] || key;
  for (const [name, value] of Object.entries(values)) {
    text = text.split(`{${name}}`).join(String(value));
  }
  return text;
}

function applyTranslations() {
  document.documentElement.lang = state.language === "zh" ? "zh-CN" : "en";
  document.title = t("app.title");

  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((element) => {
    const key = element.dataset.i18n as TranslationKey | undefined;
    if (key) {
      element.textContent = t(key);
    }
  });

  document.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>("[data-i18n-placeholder]").forEach((element) => {
    const key = element.dataset.i18nPlaceholder as TranslationKey | undefined;
    if (key) {
      element.placeholder = t(key);
    }
  });

  document.querySelectorAll<HTMLElement>("[data-i18n-aria-label]").forEach((element) => {
    const key = element.dataset.i18nAriaLabel as TranslationKey | undefined;
    if (key) {
      element.setAttribute("aria-label", t(key));
    }
  });

  document.querySelectorAll<HTMLImageElement>("[data-i18n-alt]").forEach((element) => {
    const key = element.dataset.i18nAlt as TranslationKey | undefined;
    if (key) {
      element.alt = t(key);
    }
  });

  elements.languageEn.classList.toggle("is-active", state.language === "en");
  elements.languageZh.classList.toggle("is-active", state.language === "zh");
  elements.languageEn.setAttribute("aria-pressed", String(state.language === "en"));
  elements.languageZh.setAttribute("aria-pressed", String(state.language === "zh"));
  elements.appVersion.textContent = APP_VERSION;
  renderCookiesFile(state.cookiesFile);
  updateNotice();
  updateProgressText();
}

function setLanguage(language: Language) {
  state.language = language;
  localStorage.setItem("yt-dlp-tauri-language", language);
  applyTranslations();
  if (!state.metadata) {
    renderEmptyPreview(t("preview.emptyStart"));
  }
}

function setSettingsOpen(isOpen: boolean) {
  if (isOpen) {
    // Remove hidden first so element is in the DOM, then add class for transition
    elements.settingsDrawer.hidden = false;
    elements.settingsBackdrop.hidden = false;
    // Force reflow to ensure transition triggers
    elements.settingsDrawer.getBoundingClientRect();
    elements.settingsDrawer.classList.add("is-visible");
    elements.settingsBackdrop.classList.add("is-visible");
  } else {
    elements.settingsDrawer.classList.remove("is-visible");
    elements.settingsBackdrop.classList.remove("is-visible");
    // Hide after transition ends
    const onEnd = () => {
      if (!elements.settingsDrawer.classList.contains("is-visible")) {
        elements.settingsDrawer.hidden = true;
        elements.settingsBackdrop.hidden = true;
      }
      elements.settingsDrawer.removeEventListener("transitionend", onEnd);
    };
    elements.settingsDrawer.addEventListener("transitionend", onEnd);
  }
  elements.settingsDrawer.setAttribute("aria-hidden", String(!isOpen));
  document.body.classList.toggle("settings-open", isOpen);

  if (isOpen) {
    elements.settingsClose.focus();
  } else {
    elements.settingsToggle.focus();
  }
}

function bindEvents() {
  elements.parse.addEventListener("click", () => void parseCurrentUrl());
  elements.download.addEventListener("click", () => void downloadCurrentVideo());
  elements.cancel.addEventListener("click", () => void cancelCurrentDownload());
  elements.chooseCookies.addEventListener("click", () => void chooseCookiesFile());
  elements.webLogin.addEventListener("click", () => void openLoginWindow());
  elements.clearCookies.addEventListener("click", () => void clearCookiesFile());
  elements.settingsToggle.addEventListener("click", () => setSettingsOpen(true));
  elements.settingsClose.addEventListener("click", () => setSettingsOpen(false));
  elements.settingsBackdrop.addEventListener("click", () => setSettingsOpen(false));
  elements.languageEn.addEventListener("click", () => setLanguage("en"));
  elements.languageZh.addEventListener("click", () => setLanguage("zh"));
  elements.toolsInfoBtn.addEventListener("click", () => {
    const links = `yt-dlp: https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe\nffmpeg/ffprobe: https://github.com/BtbN/FFmpeg-Builds/releases/latest\ndeno: https://github.com/denoland/deno/releases`;
    navigator.clipboard.writeText(links).then(() => {
      showToast(t("notice.linksCopied"));
    }).catch((err) => {
      showNotice(String(err), "error");
    });
  });
  elements.browseYtDlp.addEventListener("click", () => void browseTool("yt-dlp", elements.ytDlpInput));
  elements.browseFfmpeg.addEventListener("click", () => void browseTool("ffmpeg", elements.ffmpegInput));
  elements.browseFfprobe.addEventListener("click", () => void browseTool("ffprobe", elements.ffprobeInput));
  elements.browseDeno.addEventListener("click", () => void browseTool("deno", elements.denoInput));
  elements.saveTools.addEventListener("click", () => void saveToolsConfig());
  elements.openFolder.addEventListener("click", () => void openDownloadFolder());
  elements.browseFolder.addEventListener("click", () => void browseDownloadFolder());
  elements.saveFolder.addEventListener("click", () => void saveDownloadFolder());
  elements.resetFolder.addEventListener("click", () => void resetDownloadFolder());
  elements.thumbnail.addEventListener("load", () => showLoadedThumbnail());
  elements.thumbnail.addEventListener("error", () => loadNextThumbnailCandidate());
  elements.quality.addEventListener("change", () => {
    state.selectedFormat = state.metadata?.format_options[elements.quality.selectedIndex] ?? null;
    updateButtons();
  });
  elements.url.addEventListener("input", () => {
    if (elements.url.value.trim() !== state.lastUrl) {
      invalidateParsedVideo(t("preview.emptyChanged"));
    }
    updateButtons();
  });
  elements.url.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      void parseCurrentUrl();
    }
  });
  window.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && !elements.settingsDrawer.hidden) {
      setSettingsOpen(false);
    }
  });
}

function updateNotice() {
  if (state.noticeRaw) {
    elements.notice.textContent = state.noticeRaw;
  } else if (state.noticeKey) {
    elements.notice.textContent = t(state.noticeKey, state.noticeArgs);
  }
  elements.notice.className = `notice is-${state.noticeTone}`;
}

function showNotice(keyOrRaw: string, tone: NoticeTone, args: Record<string, string | number> = {}) {
  state.noticeTone = tone;
  if (translations.en[keyOrRaw] || translations.zh[keyOrRaw]) {
    state.noticeKey = keyOrRaw;
    state.noticeRaw = null;
    state.noticeArgs = args;
  } else {
    state.noticeKey = null;
    state.noticeRaw = keyOrRaw;
    state.noticeArgs = {};
  }
  updateNotice();
}

function updateProgressText() {
  if (state.progressRaw) {
    elements.progressText.textContent = state.progressRaw;
  } else if (state.currentDownloadProgress) {
    const p = state.currentDownloadProgress;
    elements.progressText.textContent = [
      p.status,
      typeof p.percent === "number" ? `${p.percent.toFixed(1)}%` : null,
      p.speed,
      p.eta ? `${t("progress.eta")} ${p.eta}` : null,
    ]
      .filter(Boolean)
      .join(" · ");
  } else if (state.progressKey) {
    elements.progressText.textContent = t(state.progressKey, state.progressArgs);
  }
}

function setProgressText(keyOrRaw: string, args: Record<string, string | number> = {}) {
  if (translations.en[keyOrRaw] || translations.zh[keyOrRaw]) {
    state.progressKey = keyOrRaw;
    state.progressRaw = null;
    state.progressArgs = args;
  } else {
    state.progressKey = null;
    state.progressRaw = keyOrRaw;
    state.progressArgs = {};
  }
  state.currentDownloadProgress = null;
  updateProgressText();
}

function showToast(message: string) {
  let toast = document.getElementById("toast");
  if (!toast) {
    toast = document.createElement("div");
    toast.id = "toast";
    document.body.appendChild(toast);
  }
  toast.textContent = message;
  toast.className = "toast";
  toast.style.display = "block";
  toast.style.opacity = "0";
  
  // Trigger reflow
  toast.getBoundingClientRect();
  toast.style.opacity = "1";
  
  setTimeout(() => {
    if (toast) {
      toast.style.opacity = "0";
      setTimeout(() => {
        if (toast) toast.style.display = "none";
      }, 300);
    }
  }, 2000);
}

async function browseTool(name: string, inputElement: HTMLInputElement) {
  try {
    const selected = await open({
      title: t("settings.chooseTool", { name }),
      directory: false,
      multiple: false,
      defaultPath: inputElement.value || undefined,
      filters: [{ name: "Executables", extensions: ["exe", "bat", "cmd", "sh", "bin"] }]
    });
    if (typeof selected === "string") {
      inputElement.value = selected;
    }
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function saveToolsConfig() {
  try {
    await invoke<AppState>("set_tools_directory", {
      directory: "",
      ytDlpPath: elements.ytDlpInput.value.trim(),
      ffmpegPath: elements.ffmpegInput.value.trim(),
      ffprobePath: elements.ffprobeInput.value.trim(),
      denoPath: elements.denoInput.value.trim(),
    });
    showNotice("notice.toolchainSaved", "success");
    await refreshTools();
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function bootstrap() {
  showNotice("notice.checkingTools", "warning");
  setProgressText("progress.idle");
  renderEmptyPreview(t("preview.emptyStart"));
  await loadAppState();
  await refreshTools();
}

async function loadAppState() {
  const appState = await invoke<AppState>("get_app_state");
  elements.folderText.textContent = appState.download_directory;
  elements.folderInput.value = appState.download_directory;

  elements.ytDlpInput.value = appState.yt_dlp_path || "";
  elements.ffmpegInput.value = appState.ffmpeg_path || "";
  elements.ffprobeInput.value = appState.ffprobe_path || "";
  elements.denoInput.value = appState.deno_path || "";
  renderCookiesFile(appState.cookies_file ?? null);
  state.cookiesStatus = appState.cookies_status as "none" | "valid" | "warning";
  if (state.cookiesStatus === "warning") {
    showNotice("notice.cookiesWarning", "warning");
  }
}

async function refreshTools() {
  setBusy(true, undefined, "tools");
  try {
    const tools = await invoke<ToolStatus[]>("check_tools");
    state.toolsReady = tools.every((tool) => tool.availability === "available");
    const toolInputMap: Record<string, HTMLInputElement> = {
      "yt-dlp": elements.ytDlpInput,
      "ffmpeg": elements.ffmpegInput,
      "ffprobe": elements.ffprobeInput,
      "deno": elements.denoInput,
    };
    for (const tool of tools) {
      const input = toolInputMap[tool.name];
      if (input && tool.availability === "available") {
        input.placeholder = tool.full_path;
        if (!input.value) {
          input.value = tool.full_path;
        }
      }
    }
    if (state.toolsReady && state.cookiesStatus === "warning") {
      showNotice("notice.cookiesWarning", "warning");
    } else {
      showNotice(state.toolsReady ? "notice.toolchainReady" : "notice.toolsMissing", state.toolsReady ? "success" : "warning");
    }
  } catch (error) {
    state.toolsReady = false;
    const message = String(error);
    showNotice(message || "settings.toolCheckFailed", "error");
  } finally {
    setBusy(false);
  }
}

async function parseCurrentUrl() {
  const url = elements.url.value.trim();
  if (!url || state.busy) {
    return;
  }

  // Intercept WeChat article URLs
  if (url.includes("mp.weixin.qq.com")) {
    showNotice("notice.wechatIntercept", "info");
    openLoginWindow();
    return;
  }

  setBusy(true, "progress.parsing", "metadata");
  renderEmptyPreview(t("preview.readingMetadata"));
  try {
    const metadata = await invoke<VideoMetadata>("parse_metadata", { url });
    state.metadata = metadata;
    state.lastUrl = url;
    state.selectedFormat = metadata.format_options[0] ?? null;
    renderMetadata(metadata);
    renderQualityOptions(metadata.format_options);
    setProgressText("progress.metadataReady");
    showNotice("notice.metadataParsed", "success");
  } catch (error) {
    renderEmptyPreview(t("preview.parseFailed"));
    setProgressText("progress.metadataFailed");
    const errMsg = String(error);
    showNotice(errMsg, "error");
    
    // Auto open login window for Bilibili rate-limiting or authentication errors
    const lowercaseErr = errMsg.toLowerCase();
    const isBilibili = url.includes("bilibili.com") || url.includes("b23.tv");
    if (
      isBilibili && (
        lowercaseErr.includes("412") ||
        lowercaseErr.includes("403") ||
        lowercaseErr.includes("401") ||
        lowercaseErr.includes("login") ||
        lowercaseErr.includes("sign in") ||
        lowercaseErr.includes("credential") ||
        lowercaseErr.includes("precondition failed")
      )
    ) {
      void openLoginWindow();
    }
  } finally {
    setBusy(false);
  }
}

async function downloadCurrentVideo() {
  const metadata = state.metadata;
  const selectedFormat = state.selectedFormat;
  const url = state.lastUrl || elements.url.value.trim();
  if (!metadata || !selectedFormat || !url || state.busy) {
    return;
  }

  setBusy(true, "progress.startingDownload", "download", { quality: selectedFormat.label });
  elements.progress.removeAttribute("value");
  try {
    const outputPath = await invoke<string | null>("download_video", {
      request: {
        url,
        format_selector: selectedFormat.format_selector,
        label: selectedFormat.label,
        title: metadata.title,
      },
    });
    elements.progress.value = 100;
    if (outputPath) {
      setProgressText("progress.savedTo", { path: outputPath });
    } else {
      setProgressText("progress.completedOpenFolder");
    }
    showNotice("notice.downloadCompleted", "success");
  } catch (error) {
    const message = String(error);
    elements.progress.value = 0;
    if (message.toLowerCase().includes("cancel")) {
      setProgressText("progress.downloadCancelled");
      showNotice("notice.downloadCancelled", "warning");
    } else {
      setProgressText("progress.downloadFailed");
      showNotice(message, "error");
    }
  } finally {
    setBusy(false);
  }
}

async function cancelCurrentDownload() {
  if (state.activeOperation !== "download" || state.cancelRequested) {
    return;
  }

  state.cancelRequested = true;
  setProgressText("progress.cancelling");
  updateButtons();
  try {
    await invoke("cancel_download");
  } catch (error) {
    showNotice(String(error), "error");
    state.cancelRequested = false;
    updateButtons();
  }
}

async function openDownloadFolder() {
  try {
    await invoke("open_download_directory");
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function browseDownloadFolder() {
  try {
    const selected = await open({
      title: t("settings.chooseFolder"),
      directory: true,
      multiple: false,
      defaultPath: elements.folderInput.value || undefined,
    });

    if (typeof selected === "string") {
      elements.folderInput.value = selected;
      await saveDownloadFolder();
    }
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function saveDownloadFolder() {
  try {
    const appState = await invoke<AppState>("set_download_directory", { directory: elements.folderInput.value });
    state.cookiesStatus = appState.cookies_status as "none" | "valid" | "warning";
    elements.folderText.textContent = appState.download_directory;
    elements.folderInput.value = appState.download_directory;
    showNotice("notice.folderUpdated", "success");
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function resetDownloadFolder() {
  try {
    const appState = await invoke<AppState>("reset_download_directory");
    state.cookiesStatus = appState.cookies_status as "none" | "valid" | "warning";
    elements.folderText.textContent = appState.download_directory;
    elements.folderInput.value = appState.download_directory;
    showNotice("notice.folderReset", "success");
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function chooseCookiesFile() {
  if (state.busy) {
    return;
  }

  try {
    const selected = await open({
      title: t("cookies.chooseFile"),
      directory: false,
      multiple: false,
      defaultPath: state.cookiesFile || undefined,
    });

    if (typeof selected === "string") {
      const appState = await invoke<AppState>("set_cookies_file", { path: selected });
      state.cookiesStatus = appState.cookies_status as "none" | "valid" | "warning";
      renderCookiesFile(appState.cookies_file ?? null);
      invalidateParsedVideo(t("preview.cookiesChanged"));
      if (state.cookiesStatus === "warning") {
        showNotice("notice.cookiesWarning", "warning");
          } else {
        showNotice("notice.cookiesUpdated", "success");
      }
    }
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function openLoginWindow() {
  if (state.busy) {
    return;
  }
  try {
    const targetUrl = elements.url.value.trim() || null;
    await invoke("open_login_window", { targetUrl });
  } catch (error) {
    showNotice(String(error), "error");
  }
}

async function onCookiesSynced() {
  await loadAppState();
  showNotice("notice.cookiesSynced", "success");
}

async function onWeChatVideoIntercepted(videoUrl: string) {
  elements.url.value = videoUrl;
  invalidateParsedVideo(t("preview.cookiesChanged"));
  await parseCurrentUrl();
}

async function clearCookiesFile() {
  if (state.busy || !state.cookiesFile) {
    return;
  }

  try {
    const appState = await invoke<AppState>("clear_cookies_file");
    state.cookiesStatus = appState.cookies_status as "none" | "valid" | "warning";
    renderCookiesFile(appState.cookies_file ?? null);
    invalidateParsedVideo(t("preview.cookiesChanged"));
    showNotice("notice.cookiesCleared", "success");
  } catch (error) {
    showNotice(String(error), "error");
  }
}

function renderMetadata(metadata: VideoMetadata) {
  elements.title.textContent = metadata.title;
  elements.details.textContent = [
    metadata.id ? `ID ${metadata.id.length > 25 ? metadata.id.slice(0, 25) + "..." : metadata.id}` : null,
    metadata.duration_seconds ? formatDuration(metadata.duration_seconds) : null,
  ]
    .filter(Boolean)
    .join(" · ");
  elements.description.textContent = metadata.description?.trim() || "";

  renderThumbnailCandidates(thumbnailUrlCandidates(metadata));
}

function renderEmptyPreview(message: string) {
  elements.title.textContent = t("preview.noVideo");
  elements.details.textContent = message;
  elements.description.textContent = "";
  clearThumbnail();
}

function invalidateParsedVideo(message: string) {
  state.metadata = null;
  state.selectedFormat = null;
  state.lastUrl = "";
  renderEmptyPreview(message);
  renderQualityOptions([]);
}

function renderThumbnailCandidates(urls: string[]) {
  state.thumbnailCandidates = urls;
  state.thumbnailCandidateIndex = 0;

  if (urls.length === 0) {
    clearThumbnail();
    return;
  }

  loadThumbnailCandidate(0);
}

function loadThumbnailCandidate(index: number) {
  const url = state.thumbnailCandidates[index];
  if (!url) {
    clearThumbnail();
    return;
  }

  state.thumbnailCandidateIndex = index;
  elements.thumbnail.dataset.thumbnailIndex = String(index);
  elements.thumbnail.hidden = true;
  elements.thumbnail.classList.remove("is-loaded");
  elements.thumbnailEmpty.hidden = false;
  elements.thumbnail.src = url;
}

function showLoadedThumbnail() {
  const currentIndex = Number(elements.thumbnail.dataset.thumbnailIndex ?? state.thumbnailCandidateIndex);
  if (!state.thumbnailCandidates[currentIndex]) {
    return;
  }

  elements.thumbnail.hidden = false;
  elements.thumbnail.classList.add("is-loaded");
  elements.thumbnailEmpty.hidden = true;
}

function loadNextThumbnailCandidate() {
  const currentIndex = Number(elements.thumbnail.dataset.thumbnailIndex ?? state.thumbnailCandidateIndex);
  const nextIndex = currentIndex + 1;
  if (nextIndex < state.thumbnailCandidates.length) {
    loadThumbnailCandidate(nextIndex);
    return;
  }

  clearThumbnail();
}

function clearThumbnail() {
  state.thumbnailCandidates = [];
  state.thumbnailCandidateIndex = 0;
  delete elements.thumbnail.dataset.thumbnailIndex;
  elements.thumbnail.removeAttribute("src");
  elements.thumbnail.classList.remove("is-loaded");
  elements.thumbnail.hidden = true;
  elements.thumbnailEmpty.hidden = false;
}

function renderQualityOptions(options: VideoFormatOption[]) {
  elements.quality.replaceChildren(
    ...options.map((option) => {
      const item = document.createElement("option");
      item.textContent = option.label;
      item.value = option.format_selector;
      return item;
    }),
  );
  elements.quality.disabled = options.length === 0;
}


function updateDownloadProgress(progress: DownloadProgress) {
  if (typeof progress.percent === "number") {
    elements.progress.value = progress.percent;
  } else {
    elements.progress.removeAttribute("value");
  }
  state.currentDownloadProgress = progress;
  state.progressKey = null;
  state.progressRaw = null;
  updateProgressText();
}

function setBusy(isBusy: boolean, progressKeyOrRaw?: string, operation: "metadata" | "download" | "tools" | null = null, args: Record<string, string | number> = {}) {
  state.busy = isBusy;
  state.activeOperation = isBusy ? operation : null;
  if (!isBusy) {
    state.cancelRequested = false;
  }
  if (progressKeyOrRaw) {
    setProgressText(progressKeyOrRaw, args);
  }
  updateButtons();
}

function renderCookiesFile(file: string | null) {
  state.cookiesFile = file?.trim() || null;
  elements.cookiesFile.textContent = state.cookiesFile ? fileNameFromPath(state.cookiesFile) : t("cookies.none");
  elements.cookiesFile.title = state.cookiesFile || t("cookies.none");
  updateButtons();
}

function updateButtons() {
  const hasUrl = elements.url.value.trim().length > 0;
  elements.parse.disabled = state.busy || !hasUrl || !state.toolsReady;
  elements.download.disabled = state.busy || !state.metadata || !state.selectedFormat || !state.toolsReady;
  elements.cancel.disabled = state.activeOperation !== "download" || state.cancelRequested;
  elements.chooseCookies.disabled = state.busy;
  elements.webLogin.disabled = state.busy;
  elements.clearCookies.disabled = state.busy || !state.cookiesFile;
  elements.browseFolder.disabled = state.busy;
  elements.saveFolder.disabled = state.busy;
  elements.resetFolder.disabled = state.busy;
  elements.browseYtDlp.disabled = state.busy;
  elements.browseFfmpeg.disabled = state.busy;
  elements.browseFfprobe.disabled = state.busy;
  elements.browseDeno.disabled = state.busy;
  elements.saveTools.disabled = state.busy;
}


function formatDuration(seconds: number) {
  const rounded = Math.max(0, Math.round(seconds));
  const hours = Math.floor(rounded / 3600);
  const minutes = Math.floor((rounded % 3600) / 60);
  const secs = rounded % 60;
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(secs).padStart(2, "0")}`
    : `${minutes}:${String(secs).padStart(2, "0")}`;
}

function fileNameFromPath(path: string) {
  return path.replace(/\\/g, "/").split("/").filter(Boolean).pop() || path;
}


bootstrap();
bindEvents();
listen<string>("wechat_video_intercepted", (event) => onWeChatVideoIntercepted(event.payload));
listen("cookies_synced", onCookiesSynced);
listen<DownloadProgress>("download_progress", (e) => updateDownloadProgress(e.payload));
