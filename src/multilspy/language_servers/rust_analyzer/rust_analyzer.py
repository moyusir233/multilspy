"""
Provides Rust specific instantiation of the LanguageServer class. Contains various configurations and settings specific to Rust.
"""

import asyncio
import json
import logging
import os
import stat
import pathlib
from contextlib import asynccontextmanager
from typing import AsyncIterator, Optional
import time

from multilspy.multilspy_logger import MultilspyLogger
from multilspy.language_server import LanguageServer
from multilspy.lsp_protocol_handler.server import ProcessLaunchInfo
from multilspy.lsp_protocol_handler.lsp_types import InitializeParams
from multilspy.multilspy_config import MultilspyConfig
from multilspy.multilspy_utils import FileUtils
from multilspy.multilspy_utils import PlatformUtils


class RustAnalyzer(LanguageServer):
    """
    Provides Rust specific instantiation of the LanguageServer class. Contains various configurations and settings specific to Rust.
    """

    def __init__(
        self,
        config: MultilspyConfig,
        logger: MultilspyLogger,
        repository_root_path: str,
    ):
        """
        Creates a RustAnalyzer instance. This class is not meant to be instantiated directly. Use LanguageServer.create() instead.
        """
        rustanalyzer_executable_path = self.setup_runtime_dependencies(logger, config)
        super().__init__(
            config,
            logger,
            repository_root_path,
            ProcessLaunchInfo(
                cmd=rustanalyzer_executable_path, cwd=repository_root_path
            ),
            "rust",
        )
        self.server_ready = asyncio.Event()
        self.wait_work_done_progress_create_max_time: float = 0.0
        self.uncomplete_work_done_progress: set[str] = set()

    def setup_runtime_dependencies(
        self, logger: MultilspyLogger, config: MultilspyConfig
    ) -> str:
        """
        Setup runtime dependencies for rust_analyzer.
        """
        import subprocess

        # try get `rust-analyzer` executable path from `which rust-analyzer`
        try:
            process = subprocess.run(
                ["bash", "-c", "which rust-analyzer"],
                check=True,
                capture_output=True,
                text=True,
            )

            if len(process.stdout) != 0:
                ra_path = process.stdout.strip()
                logger.log(
                    f"Found rust-analyzer executable path: {ra_path}.",
                    logging.INFO,
                )
                return ra_path
        except subprocess.CalledProcessError as e:
            logger.log(
                f"Failed to run `which rust-analyzer`: {e}",
                logging.ERROR,
            )
            raise e

        platform_id = PlatformUtils.get_platform_id()

        with open(
            os.path.join(os.path.dirname(__file__), "runtime_dependencies.json"), "r"
        ) as f:
            d = json.load(f)
            del d["_description"]

        # assert platform_id.value in [
        #     "linux-x64",
        #     "win-x64",
        # ], "Only linux-x64 and win-x64 platform is supported for in multilspy at the moment"

        runtime_dependencies = d["runtimeDependencies"]
        runtime_dependencies = [
            dependency
            for dependency in runtime_dependencies
            if dependency["platformId"] == platform_id.value
        ]
        assert len(runtime_dependencies) == 1
        dependency = runtime_dependencies[0]

        rustanalyzer_ls_dir = os.path.join(
            os.path.dirname(__file__), "static", "RustAnalyzer"
        )
        rustanalyzer_executable_path = os.path.join(
            rustanalyzer_ls_dir, dependency["binaryName"]
        )
        if not os.path.exists(rustanalyzer_executable_path):
            os.makedirs(rustanalyzer_ls_dir, exist_ok=True)
            logger.log(
                f"Downloading Rust Analyzer to {rustanalyzer_ls_dir}.",
                logging.INFO,
            )
            if dependency["archiveType"] == "gz":
                FileUtils.download_and_extract_archive(
                    logger,
                    dependency["url"],
                    rustanalyzer_executable_path,
                    dependency["archiveType"],
                )
            else:
                FileUtils.download_and_extract_archive(
                    logger,
                    dependency["url"],
                    rustanalyzer_ls_dir,
                    dependency["archiveType"],
                )
        assert os.path.exists(rustanalyzer_executable_path)
        os.chmod(rustanalyzer_executable_path, stat.S_IEXEC)

        logger.log(
            f"Rust Analyzer executable path: {rustanalyzer_executable_path}.",
            logging.INFO,
        )
        return rustanalyzer_executable_path

    def _get_initialize_params(self, repository_absolute_path: str) -> InitializeParams:
        """
        Returns the initialize params for the Rust Analyzer Language Server.
        """
        with open(
            os.path.join(os.path.dirname(__file__), "initialize_params.json"), "r"
        ) as f:
            d = json.load(f)

        del d["_description"]

        d["processId"] = os.getpid()
        assert d["rootPath"] == "$rootPath"
        d["rootPath"] = repository_absolute_path

        assert d["rootUri"] == "$rootUri"
        d["rootUri"] = pathlib.Path(repository_absolute_path).as_uri()

        assert d["workspaceFolders"][0]["uri"] == "$uri"
        d["workspaceFolders"][0]["uri"] = pathlib.Path(
            repository_absolute_path
        ).as_uri()

        assert d["workspaceFolders"][0]["name"] == "$name"
        d["workspaceFolders"][0]["name"] = os.path.basename(repository_absolute_path)

        return d

    @asynccontextmanager
    async def start_server(
        self, wait_work_progress_done_create_time_window_seconds: Optional[int] = 300
    ) -> AsyncIterator["RustAnalyzer"]:
        """
        Starts the Rust Analyzer Language Server, waits for the server to be ready and yields the LanguageServer instance.

        Args:
            wait_work_progress_done_create_time_window_seconds (int, optional): The time window in seconds to wait for the work done progress create request from server. Defaults to 300. If pass, client will wait all created work progress done progress which is created in the time window, by wait the server '$/progress' notification.

        Usage:
        ```
        async with lsp.start_server():
            # LanguageServer has been initialized and ready to serve requests
            await lsp.request_definition(...)
            await lsp.request_references(...)
            # Shutdown the LanguageServer on exit from scope
        # LanguageServer has been shutdown
        """

        async def register_capability_handler(params):
            assert "registrations" in params
            for registration in params["registrations"]:
                if registration["method"] == "workspace/executeCommand":
                    self.initialize_searcher_command_available.set()
                    self.resolve_main_method_available.set()
            return

        async def lang_status_handler(params):
            # TODO: Should we wait for
            # server -> client: {'jsonrpc': '2.0', 'method': 'language/status', 'params': {'type': 'ProjectStatus', 'message': 'OK'}}
            # Before proceeding?
            if params["type"] == "ServiceReady" and params["message"] == "ServiceReady":
                self.service_ready_event.set()

        async def execute_client_command_handler(params):
            return []

        async def do_nothing(params):
            return

        async def check_experimental_status(params):
            self.logger.log(f"LSP: window/experimentalStatus: {params}", logging.DEBUG)
            if (
                params["quiescent"] == True
                and len(self.uncomplete_work_done_progress) == 0
                and time.time() >= self.wait_work_done_progress_create_max_time
            ):
                self.server_ready.set()

        async def window_log_message(msg):
            self.logger.log(f"LSP: window/logMessage: {msg}", logging.INFO)

        async def create_work_done_progress(params):
            self.logger.log(
                f"LSP: window/workDoneProgress/create: {params}", logging.DEBUG
            )

            # 接收到请求时，如果还在等待work done progress create的时间窗口内，那么
            # 将params中的token保存
            if (
                time.time() < self.wait_work_done_progress_create_max_time
                and params.get("token") is not None
            ):
                self.logger.log(
                    f"Received and save work done progress create request with token: {params['token']}",
                    logging.DEBUG,
                )
                self.uncomplete_work_done_progress.add(params["token"])

            return

        async def progress_handler(params):
            self.logger.log(f"LSP: $/progress: {params}", logging.DEBUG)

            # 接收到progress notification时，如果params中的token在self.uncomplete_work_done_progress中，那么
            # 则将该token从self.uncomplete_work_done_progress中移除
            if (
                params.get("token") is not None
                and params.get("value") is not None
                and params["value"].get("kind") == "end"
                and params["token"] in self.uncomplete_work_done_progress
            ):
                self.uncomplete_work_done_progress.remove(params["token"])
                self.logger.log(
                    f"Received work done progress end notification with token: {params['token']}, remain uncomplete_work_done_progress: {self.uncomplete_work_done_progress}",
                    logging.DEBUG,
                )

                if (
                    len(self.uncomplete_work_done_progress) == 0
                    and time.time() >= self.wait_work_done_progress_create_max_time
                ):
                    if not self.server_ready.is_set():
                        self.server_ready.set()

        async def check_status_and_set_ready(
            num_epoch: int, sleep_time_secs_per_epoch: float
        ):
            for i in range(1, num_epoch + 1):
                await asyncio.sleep(sleep_time_secs_per_epoch)
                self.logger.log(
                    f"check server status, uncomplete_work_done_progress: {self.uncomplete_work_done_progress}",
                    logging.DEBUG,
                )
                # 每次醒后，检查下是否可以直接ready
                if (
                    time.time() >= self.wait_work_done_progress_create_max_time
                    and len(self.uncomplete_work_done_progress) == 0
                ):
                    self.logger.log(
                        f"Server is ready after {i * sleep_time_secs_per_epoch} seconds",
                        logging.DEBUG,
                    )
                    self.server_ready.set()
                    return

            if not self.server_ready.is_set():
                self.logger.log(
                    f"Server is not ready after {num_epoch * sleep_time_secs_per_epoch} seconds, set server_ready to True",
                    logging.WARNING,
                )
                self.server_ready.set()

        if wait_work_progress_done_create_time_window_seconds is not None:
            # 当前时间戳加上该等待的秒数，获得等待的结束时间
            self.wait_work_done_progress_create_max_time = (
                time.time() + wait_work_progress_done_create_time_window_seconds
            )

        self.server.on_request("client/registerCapability", register_capability_handler)
        self.server.on_request(
            "window/workDoneProgress/create",
            create_work_done_progress,
        )
        self.server.on_request(
            "workspace/executeClientCommand", execute_client_command_handler
        )

        self.server.on_notification("language/status", lang_status_handler)
        self.server.on_notification("window/logMessage", window_log_message)
        self.server.on_notification("$/progress", progress_handler)
        self.server.on_notification("textDocument/publishDiagnostics", do_nothing)
        self.server.on_notification("language/actionableNotification", do_nothing)
        self.server.on_notification(
            "experimental/serverStatus", check_experimental_status
        )

        async with super().start_server(
            wait_work_progress_done_create_time_window_seconds=wait_work_progress_done_create_time_window_seconds
        ):
            self.logger.log("Starting RustAnalyzer server process", logging.INFO)
            await self.server.start()
            initialize_params = self._get_initialize_params(self.repository_root_path)

            self.logger.log(
                "Sending initialize request from LSP client to LSP server and awaiting response",
                logging.INFO,
            )
            init_response = await self.server.send.initialize(initialize_params)
            assert init_response["capabilities"]["textDocumentSync"]["change"] == 2
            assert "completionProvider" in init_response["capabilities"]
            assert init_response["capabilities"]["completionProvider"] == {
                "resolveProvider": True,
                "triggerCharacters": [":", ".", "'", "("],
                "completionItem": {"labelDetailsSupport": True},
            }
            self.server.notify.initialized({})
            self.completions_available.set()

            self.logger.log("Wait the server to be ready", logging.INFO)

            # 避免永久沉睡，设置固定20分钟的超时时间
            _task = asyncio.create_task(
                check_status_and_set_ready(
                    num_epoch=40,
                    sleep_time_secs_per_epoch=30,
                )
            )

            await self.server_ready.wait()

            self.logger.log("RustAnalyzer server is ready", logging.INFO)

            try:
                yield self
            finally:
                await self.server.shutdown()
                await self.server.stop()
