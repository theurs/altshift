# AltShift

Windows keyboard layout switcher for Alt+Shift. Runs silently in the background.

**Run as Administrator** — required to work with apps that have higher integrity level.

## Add to Autostart

To run AltShift automatically on Windows login, create a scheduled task:

```cmd
schtasks /create /tn "SuperAltShift" /tr "C:\Полный\Путь\К\Твоему\altshift.exe" /sc onlogon /rl highest /f
```

To remove the task:

```cmd
schtasks /delete /tn "SuperAltShift" /f
```

## Disable Default Layout Switching

Windows uses Alt+Shift for its own layout switching by default. To disable it and avoid conflicts:

```cmd
reg add "HKCU\Keyboard Layout\Toggle" /v "Language Hotkey" /t REG_SZ /d 3 /f
```

After running this command, **log out and back in** (or reboot) for the change to take effect.

To restore the default Alt+Shift switching:

```cmd
reg delete "HKCU\Keyboard Layout\Toggle" /v "Language Hotkey" /f
```

---

## RU

Переключатель раскладки клавиатуры по Alt+Shift для Windows. Работает тихо в фоне, без иконок и меню.

**Запуск от имени Администратора** — необходим для работы с приложениями повышенного уровня целостности (RDP, системные утилиты).

## Скачать

Последний `altshift.exe` — в разделе [Releases](../../releases).

## Добавить в автозапуск

Чтобы AltShift запускался автоматически при входе в Windows, создайте запланированную задачу:

```cmd
schtasks /create /tn "SuperAltShift" /tr "C:\Полный\Путь\К\Твоему\altshift.exe" /sc onlogon /rl highest /f
```

- `/tn "SuperAltShift"` — имя задачи
- `/tr "..."` — полный путь к `altshift.exe`
- `/sc onlogon` — запуск при входе пользователя
- `/rl highest` — запуск с наивысшими привилегиями (Администратор)
- `/f` — принудительное создание без подтверждения

Для удаления задачи:

```cmd
schtasks /delete /tn "SuperAltShift" /f

## Отключить стандартное переключение раскладки

Windows по умолчанию использует Alt+Shift для переключения раскладки. Чтобы отключить это и избежать конфликтов:

```cmd
reg add "HKCU\Keyboard Layout\Toggle" /v "Language Hotkey" /t REG_SZ /d 3 /f
```

После выполнения команды **выйдите из системы и войдите заново** (или перезагрузитесь), чтобы изменения вступили в силу.

Чтобы вернуть стандартное переключение Alt+Shift:

```cmd
reg delete "HKCU\Keyboard Layout\Toggle" /v "Language Hotkey" /f
```
