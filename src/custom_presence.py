"""
Custom Presence management module.
"""

import os
import json
from typing import Optional, Dict, Any, List
from src.rpc import ClientRPC, ActivityType
from src.logger import logger
from src.constants import config
from src.utils import is_valid_url




class CustomPresence:
    """
    Manages custom Discord Rich Presence activities.
    """

    def __init__(self, client_id: Optional[str] = None, debug: bool = False):
        """
        Initialize the custom presence manager.
        """
        self.client_id = client_id
        self.debug = debug
        self.rpc: Optional[ClientRPC] = None
        self._is_active = False
        self._current_data: Dict[str, Any] = {}
        self._current_preset_name: Optional[str] = None

        logger.info("Initialized with client_id=%s", client_id or "default")

    @property
    def is_active(self) -> bool:
        """Check if custom presence is currently active."""
        return self._is_active

    @property
    def is_connected(self) -> bool:
        """Check if RPC connection is established."""
        if self.rpc is None:
            return False
        try:
            return getattr(self.rpc, "_ClientRPC__connected", False)
        except AttributeError:
            return False

    def connect(self, client_id: Optional[str] = None) -> bool:
        """
        Establish connection to Discord RPC.
        """
        if self.is_connected:
            logger.warning("Custom presence already connected")
            return True

        try:
            if isinstance(client_id, dict):
                client_id = client_id.get("client_id")

            app_id = client_id or self.client_id
            if not app_id:
                raise ValueError("Client ID is required to connect")

            app_id = str(app_id).strip()

            logger.info("Connecting custom presence with client_id=%s", app_id)
            self.rpc = ClientRPC(client_id=app_id, debug=self.debug)
            self.rpc.connect()
            self._is_active = True
            self.client_id = app_id

            logger.info("Custom presence connected successfully")
            return True

        except Exception as exc:
            logger.error("Failed to connect custom presence: %s", exc)
            self.rpc = None
            self._is_active = False
            return False

    # pylint: disable=too-many-arguments
    def update(
        self,
        state: Optional[str] = None,
        details: Optional[str] = None,
        activity_type: int = 0,
        start_time: Optional[int] = None,
        end_time: Optional[int] = None,
        large_image: Optional[str] = None,
        large_text: Optional[str] = None,
        small_image: Optional[str] = None,
        small_text: Optional[str] = None,
        buttons: Optional[list] = None,
    ) -> bool:
        """
        Update the custom Discord Rich Presence activity.
        """
        if isinstance(state, dict):
            data = state
            state = data.get("state")
            details = data.get("details")
            activity_type = data.get("activity_type", 0)
            start_time = data.get("start_time")
            end_time = data.get("end_time")
            large_image = data.get("large_image")
            large_text = data.get("large_text")
            small_image = data.get("small_image")
            small_text = data.get("small_text")
            buttons = data.get("buttons")

        if not self.is_connected:
            logger.error("Cannot update: custom presence not connected")
            return False

        if self.rpc is None:
            logger.error("RPC client is None")
            return False

        try:
            validated_buttons = None
            if buttons:
                validated_buttons = []
                for btn in buttons[:2]:
                    if isinstance(btn, dict):
                        label = btn.get("label", "").strip()
                        url = btn.get("url", "").strip()

                        if not label:
                            logger.warning("Button missing label, skipping")
                            continue

                        if not url:
                            logger.warning("Button %s missing URL, skipping", label)
                            continue

                        if not is_valid_url(url):
                            logger.warning(
                                "Button %s has invalid URL: %s, skipping", label, url
                            )
                            continue

                        validated_buttons.append({"label": label, "url": url})

                if not validated_buttons:
                    validated_buttons = None

            state_clean = state.strip() if state and isinstance(state, str) else None
            details_clean = (
                details.strip() if details and isinstance(details, str) else None
            )
            large_image_clean = (
                large_image.strip()
                if large_image and isinstance(large_image, str)
                else None
            )
            large_text_clean = (
                large_text.strip()
                if large_text and isinstance(large_text, str)
                else None
            )
            small_image_clean = (
                small_image.strip()
                if small_image and isinstance(small_image, str)
                else None
            )
            small_text_clean = (
                small_text.strip()
                if small_text and isinstance(small_text, str)
                else None
            )

            # Validate field lengths (Discord limit: 128 characters)
            if state_clean and len(state_clean) > 128:
                raise ValueError(
                    f"State exceeds 128 characters (current: {len(state_clean)})"
                )
            if details_clean and len(details_clean) > 128:
                raise ValueError(
                    f"Details exceeds 128 characters (current: {len(details_clean)})"
                )
            if large_image_clean and len(large_image_clean) > 128:
                raise ValueError(
                    f"Large image exceeds 128 characters (current: {len(large_image_clean)})"
                )
            if large_text_clean and len(large_text_clean) > 128:
                raise ValueError(
                    f"Large text exceeds 128 characters (current: {len(large_text_clean)})"
                )
            if small_image_clean and len(small_image_clean) > 128:
                raise ValueError(
                    f"Small image exceeds 128 characters (current: {len(small_image_clean)})"
                )
            if small_text_clean and len(small_text_clean) > 128:
                raise ValueError(
                    f"Small text exceeds 128 characters (current: {len(small_text_clean)})"
                )

            if validated_buttons:
                for btn in validated_buttons:
                    if len(btn["label"]) > 32:
                        raise ValueError(
                            f"Button label exceeds 32 characters (current: {len(btn['label'])})"
                        )
                    if len(btn["url"]) > 512:
                        raise ValueError(
                            f"Button URL exceeds 512 characters (current: {len(btn['url'])})"
                        )

            if not state_clean and not details_clean:
                raise ValueError("At least state or details must be provided")

            activity_enum = None
            if activity_type == 0:
                activity_enum = ActivityType.PLAYING
            elif activity_type == 2:
                activity_enum = ActivityType.LISTENING
            elif activity_type == 3:
                activity_enum = ActivityType.WATCHING
            elif activity_type == 5:
                activity_enum = ActivityType.COMPETING

            logger.info(
                "Updating custom presence: state=%s, details=%s, activity_type=%s",
                state_clean,
                details_clean,
                activity_type,
            )

            self.rpc.update(
                state=state_clean,
                details=details_clean,
                activity_type=activity_enum,
                start_time=start_time,
                end_time=end_time,
                large_image=large_image_clean,
                large_text=large_text_clean,
                small_image=small_image_clean,
                small_text=small_text_clean,
                buttons=validated_buttons,
            )

            # Store current data for retrieval (with original values)
            self._current_data = {
                "state": state_clean,
                "details": details_clean,
                "activity_type": activity_type,
                "start_time": start_time,
                "end_time": end_time,
                "large_image": large_image_clean,
                "large_text": large_text_clean,
                "small_image": small_image_clean,
                "small_text": small_text_clean,
                "buttons": validated_buttons,
            }

            logger.info("Custom presence updated successfully")
            return True

        except Exception as exc:
            raise ValueError(exc) from exc

    def clear(self) -> bool:
        """
        Clear the current custom presence activity.
        """
        if not self.is_connected:
            logger.warning("Cannot clear: custom presence not connected")
            return False

        if self.rpc is None:
            logger.error("RPC client is None")
            return False

        try:
            logger.info("Clearing custom presence activity")
            self.rpc.clear_activity()
            self._current_data = {}
            self._current_preset_name = None
            logger.info("Custom presence cleared successfully")
            return True

        except Exception as exc:
            logger.error("Failed to clear custom presence: %s", exc)
            return False

    def disconnect(self) -> bool:
        """
        Close the RPC connection and stop custom presence.
        """
        if not self.is_connected:
            logger.warning("Custom presence not connected")
            return True

        if self.rpc is None:
            logger.warning("RPC client is None")
            return True

        try:
            logger.info("Disconnecting custom presence")
            self.rpc.close()
            self.rpc = None
            self._is_active = False
            self._current_data = {}
            logger.info(
                "Custom presence disconnected successfully (preset %s retained)",
                self._current_preset_name or "none",
            )
            return True

        except Exception as exc:
            logger.error("Failed to disconnect custom presence: %s", exc)
            return False

    def get_current_data(self) -> Dict[str, Any]:
        """
        Get the current custom presence data.
        """
        return self._current_data.copy()

    def get_status(self) -> Dict[str, Any]:
        """
        Get the current status of custom presence.
        """
        return {
            "active": self._is_active,
            "connected": self.is_connected,
            "client_id": self.client_id,
            "current_data": self.get_current_data(),
        }

    # Preset management methods

    @staticmethod
    def _load_presets_file() -> Dict[str, Dict[str, Any]]:
        """Load presets from JSON file."""
        if not os.path.isfile(config.custom_presets_path):
            logger.info("Presets file not found, returning empty dict")
            return {}

        try:
            with open(config.custom_presets_path, "r", encoding="utf-8") as f:
                data = json.load(f)
                logger.info("Loaded %d presets from file", len(data))
                return data
        except Exception as exc:
            logger.error("Failed to load presets file: %s", exc)
            return {}

    @staticmethod
    def _save_presets_file(presets: Dict[str, Dict[str, Any]]) -> bool:
        """Save presets to JSON file."""
        try:
            directory = os.path.dirname(config.custom_presets_path) or str(config.base_dir.parent)
            os.makedirs(directory, exist_ok=True)

            with open(config.custom_presets_path, "w", encoding="utf-8") as f:
                json.dump(presets, f, indent=4, ensure_ascii=False)

            logger.info("Saved %d presets to file", len(presets))
            return True
        except Exception as exc:
            logger.error("Failed to save presets file: %s", exc)
            return False

    # pylint: disable=too-many-arguments
    def save_preset(
        self,
        preset_name: str,
        client_id: str,
        state: Optional[str] = None,
        details: Optional[str] = None,
        activity_type: int = 0,
        start_time: Optional[int] = None,
        end_time: Optional[int] = None,
        large_image: Optional[str] = None,
        large_text: Optional[str] = None,
        small_image: Optional[str] = None,
        small_text: Optional[str] = None,
        buttons: Optional[list] = None,
    ) -> bool:
        """
        Save a custom presence preset.
        """
        if not preset_name or not preset_name.strip():
            logger.error("Preset name cannot be empty")
            raise ValueError("Preset name is required to save preset")

        if not client_id or not client_id.strip():
            logger.error("Client ID cannot be empty")
            raise ValueError("Client ID is required to save preset")

        try:
            validated_buttons = None
            if buttons:
                validated_buttons = []
                for btn in buttons[:2]:  # Max 2 buttons
                    if isinstance(btn, dict):
                        label = btn.get("label", "").strip()
                        url = btn.get("url", "").strip()

                        if not label:
                            logger.warning("Button missing label, skipping in preset")
                            continue

                        if not url:
                            logger.warning(
                                "Button %s missing URL, skipping in preset", label
                            )
                            continue

                        if not is_valid_url(url):
                            logger.warning(
                                "Button %s has invalid URL: %s, skipping in preset",
                                label,
                                url,
                            )
                            continue

                        validated_buttons.append({"label": label, "url": url})

                if not validated_buttons:
                    validated_buttons = None

            presets = self._load_presets_file()

            preset_data = {
                "client_id": client_id,
                "state": state,
                "details": details,
                "activity_type": activity_type,
                "start_time": start_time,
                "end_time": end_time,
                "large_image": large_image,
                "large_text": large_text,
                "small_image": small_image,
                "small_text": small_text,
                "buttons": validated_buttons,
            }

            presets[preset_name] = preset_data

            if self._save_presets_file(presets):
                logger.info("Preset %s saved successfully", preset_name)
                return True
            return False

        except Exception as exc:
            logger.error("Failed to save preset %s: %s", preset_name, exc)
            raise ValueError(exc) from exc

    def load_preset(self, preset_name: str) -> Optional[Dict[str, Any]]:
        """
        Load a custom presence preset by name.
        """
        try:
            presets = self._load_presets_file()
            preset = presets.get(preset_name)

            if preset:
                logger.info("Loaded preset %s", preset_name)
            else:
                logger.warning("Preset %s not found", preset_name)

            return preset

        except Exception as exc:
            logger.error("Failed to load preset %s: %s", preset_name, exc)
            raise ValueError(exc) from exc

    def apply_preset(self, preset_name: str, connect: bool = False) -> bool:
        """
        Apply a saved preset to the current custom presence.
        """
        try:
            preset = self.load_preset(preset_name)
            if not preset:
                logger.error("Cannot apply preset %s: not found", preset_name)
                return False
            self._current_preset_name = preset_name
            client_id = preset.get("client_id")
            if not client_id:
                logger.error("Preset %s has no client_id", preset_name)
                return False
            self._current_data = {
                "state": preset.get("state"),
                "details": preset.get("details"),
                "activity_type": preset.get("activity_type", 0),
                "start_time": preset.get("start_time"),
                "end_time": preset.get("end_time"),
                "large_image": preset.get("large_image"),
                "large_text": preset.get("large_text"),
                "small_image": preset.get("small_image"),
                "small_text": preset.get("small_text"),
                "buttons": preset.get("buttons"),
            }
            if connect:
                if self.client_id != client_id or not self.is_connected:
                    if self.is_connected:
                        self.disconnect()

                    if not self.connect(client_id=client_id):
                        logger.error(
                            "Failed to connect with client_id from preset %s",
                            preset_name,
                        )
                        return False
                success = self.update(
                    state=self._current_data.get("state"),
                    details=self._current_data.get("details"),
                    activity_type=self._current_data.get("activity_type", 0),
                    start_time=self._current_data.get("start_time"),
                    end_time=self._current_data.get("end_time"),
                    large_image=self._current_data.get("large_image"),
                    large_text=self._current_data.get("large_text"),
                    small_image=self._current_data.get("small_image"),
                    small_text=self._current_data.get("small_text"),
                    buttons=self._current_data.get("buttons"),
                )
                if not success:
                    logger.error("Failed to update RPC for preset %s", preset_name)
                    return False
            logger.info(
                "Applied preset %s successfully (connect=%s)", preset_name, connect
            )
            return True

        except Exception as exc:
            logger.error("Failed to apply preset %s: %s", preset_name, exc)
            return False

    def list_presets(self) -> List[Dict[str, Any]]:
        """
        List all saved custom presence presets.
        """
        try:
            presets = self._load_presets_file()

            result = [
                {
                    "name": name,
                    "client_id": data.get("client_id"),
                    "state": data.get("state"),
                    "details": data.get("details"),
                    "activity_type": data.get("activity_type", 0),
                }
                for name, data in presets.items()
            ]

            logger.info("Listed %d presets", len(result))
            return result

        except Exception as exc:
            logger.error("Failed to list presets: %s", exc)
            return []

    def delete_preset(self, preset_name: str) -> bool:
        """
        Delete a custom presence preset.
        """
        try:
            presets = self._load_presets_file()

            if preset_name not in presets:
                logger.warning("Preset %s not found, cannot delete", preset_name)
                return False

            del presets[preset_name]

            if self._save_presets_file(presets):
                logger.info("Preset %s deleted successfully", preset_name)
                return True
            return False

        except Exception as exc:
            logger.error("Failed to delete preset %s: %s", preset_name, exc)
            return False

    def get_preset_details(self, preset_name: str) -> Optional[Dict[str, Any]]:
        """
        Get full details of a specific preset.
        """
        preset = self.load_preset(preset_name)
        if preset:
            preset["name"] = preset_name
        return preset

    def get_current_preset_name(self) -> Optional[str]:
        """
        Get the name of the currently active preset.
        """
        logger.info("Current active preset: %s", self._current_preset_name)
        return self._current_preset_name

    def apply_current_data(self) -> bool:
        """
        Apply the currently stored data to the RPC.
        """
        if not self.is_connected:
            logger.error("Cannot apply data: not connected")
            return False

        if not self._current_data:
            logger.warning("No current data to apply")
            return False

        try:
            logger.info("Applying current data to RPC")
            return self.update(
                state=self._current_data.get("state"),
                details=self._current_data.get("details"),
                activity_type=self._current_data.get("activity_type", 0),
                start_time=self._current_data.get("start_time"),
                end_time=self._current_data.get("end_time"),
                large_image=self._current_data.get("large_image"),
                large_text=self._current_data.get("large_text"),
                small_image=self._current_data.get("small_image"),
                small_text=self._current_data.get("small_text"),
                buttons=self._current_data.get("buttons"),
            )
        except Exception as exc:
            logger.error("Failed to apply current data: %s", exc)
            return False

    def update_current_preset(self) -> bool:
        """
        Update the currently active preset with the current presence data.
        """
        if not self._current_preset_name:
            logger.error("No active preset to update")
            return False

        if not self.is_connected:
            logger.error("Cannot update preset: not connected")
            return False

        if not self._current_data:
            logger.error("No current data to save")
            return False

        try:
            logger.info(
                "Updating current preset %s with current data",
                self._current_preset_name,
            )

            current_client_id = self.client_id
            if not current_client_id:
                logger.error("No client_id available")
                return False

            success = self.save_preset(
                preset_name=self._current_preset_name,
                client_id=current_client_id,
                state=self._current_data.get("state"),
                details=self._current_data.get("details"),
                activity_type=self._current_data.get("activity_type", 0),
                start_time=self._current_data.get("start_time"),
                end_time=self._current_data.get("end_time"),
                large_image=self._current_data.get("large_image"),
                large_text=self._current_data.get("large_text"),
                small_image=self._current_data.get("small_image"),
                small_text=self._current_data.get("small_text"),
                buttons=self._current_data.get("buttons"),
            )

            if success:
                logger.info(
                    "Current preset %s updated successfully",
                    self._current_preset_name,
                )
            else:
                logger.error(
                    "Failed to update current preset %s", self._current_preset_name
                )

            return success

        except Exception as exc:
            logger.error("Failed to update current preset: %s", exc)
            return False
