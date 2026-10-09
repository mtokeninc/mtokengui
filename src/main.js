const tauriCore = window.__TAURI__?.core;
const tauriWindow = window.__TAURI__?.window;
const appWindow = tauriWindow ? tauriWindow.getCurrentWindow() : null;
let settingsItems = [];
let selectedSettingIndex = null;
let editingSettingIndex = null;

const storage = {
  theme: "mtoken-theme",
  notes: "mtoken-notes",
};

function applyTheme(theme) {
  document.body.dataset.theme = theme;
  localStorage.setItem(storage.theme, theme);
}

async function loadSystemInfo() {
  const systemInfo = document.querySelector("#system-info");
  if (!tauriCore || !tauriCore.invoke) {
    systemInfo.textContent = navigator.platform || "Unknown system";
    return;
  }

  try {
    const status = await tauriCore.invoke("get_app_status");
    systemInfo.textContent = status.system;
  } catch {
    systemInfo.textContent = navigator.platform || "Unknown system";
  }
}

function renderSettingsList() {
  const rows = document.querySelector("#settings-rows");
  rows.replaceChildren();

  settingsItems.forEach((setting, index) => {
    const row = document.createElement("tr");
    row.dataset.index = String(index);
    row.setAttribute("aria-selected", String(index === selectedSettingIndex));
    if (index === selectedSettingIndex) row.classList.add("selected");

    const nameCell = document.createElement("td");
    nameCell.textContent = setting.key;
    const valueCell = document.createElement("td");
    valueCell.textContent = setting.value;
    row.append(nameCell, valueCell);
    rows.append(row);
  });

  document.querySelector('[data-action="edit"]').disabled = selectedSettingIndex === null;
  document.querySelector('[data-action="delete"]').disabled = selectedSettingIndex === null;
}

async function loadSettingsFromRust() {
  if (!tauriCore || !tauriCore.invoke) {
    settingsItems = [];
    renderSettingsList();
    return;
  }

  try {
    settingsItems = await tauriCore.invoke("read_settings");
    renderSettingsList();
    document.querySelector("#settings-status").textContent = "";
  } catch (error) {
    settingsItems = [];
    renderSettingsList();
    document.querySelector("#settings-status").textContent = `Could not load records.csv: ${error}`;
  }
}

function openSettingsEditor(index = null) {
  editingSettingIndex = index;
  const setting = index === null ? { key: "", value: "" } : settingsItems[index];
  document.querySelector("#settings-editor-title").textContent = index === null ? "Add entry" : "Edit entry";
  document.querySelector("#setting-name").value = setting.key;
  document.querySelector("#setting-value").value = setting.value;
  document.querySelector("#settings-editor").showModal();
}

async function submitSettingsToRust() {
  if (!tauriCore || !tauriCore.invoke) {
    window.alert("Saving requires the desktop app.");
    return;
  }

  try {
    const result = await tauriCore.invoke("submit_settings", { settings: settingsItems });
    document.querySelector("#settings-status").textContent = result.message || "Settings saved.";
  } catch (error) {
    document.querySelector("#settings-status").textContent = `Submit failed: ${error}`;
  }
}

window.addEventListener("DOMContentLoaded", () => {
  const savedTheme = localStorage.getItem(storage.theme) || "dark";
  applyTheme(savedTheme);
  loadSettingsFromRust();
  loadSystemInfo();

  document.querySelector("#settings-list").addEventListener("click", (event) => {
    const row = event.target.closest("tr[data-index]");
    if (!row) return;

    selectedSettingIndex = Number(row.dataset.index);
    renderSettingsList();
  });

  document.querySelector("#settings-form").addEventListener("submit", (event) => {
    event.preventDefault();
    const item = {
      key: document.querySelector("#setting-name").value.trim(),
      value: document.querySelector("#setting-value").value.trim(),
    };
    if (editingSettingIndex === null) {
      settingsItems.push(item);
      selectedSettingIndex = settingsItems.length - 1;
    } else {
      settingsItems[editingSettingIndex] = item;
      selectedSettingIndex = editingSettingIndex;
    }
    document.querySelector("#settings-editor").close();
    renderSettingsList();
  });

  document.querySelector("#settings-cancel").addEventListener("click", () => {
    document.querySelector("#settings-editor").close();
  });

  document.querySelectorAll(".action-button").forEach((button) => {
    button.addEventListener("click", () => {
      const action = button.dataset.action;

      if (action === "submit") {
        submitSettingsToRust();
      } else if (action === "add") {
        openSettingsEditor();
      } else if (action === "edit" && selectedSettingIndex !== null) {
        openSettingsEditor(selectedSettingIndex);
      } else if (action === "delete" && selectedSettingIndex !== null) {
        settingsItems.splice(selectedSettingIndex, 1);
        selectedSettingIndex = null;
        renderSettingsList();
      }
    });
  });

  if (appWindow) {
    document.querySelector("#minimize-button").addEventListener("click", () => appWindow.minimize());
    document.querySelector("#maximize-button").addEventListener("click", () => appWindow.toggleMaximize());
    document.querySelector("#close-button").addEventListener("click", () => appWindow.close());
  }

  document.querySelector("#workspace-close-button").addEventListener("click", () => {
    if (appWindow) {
      appWindow.close();
    } else {
      window.close();
    }
  });
});
