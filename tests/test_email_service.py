"""
Unit tests for email-indirect service & SMTP email handling.
"""

import sys
import unittest
from pathlib import Path
from unittest.mock import patch, MagicMock

# Add repository root and service directory to sys.path for test execution
repo_root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(repo_root))
sys.path.insert(0, str(repo_root / "email-indirect" / "service"))

# Mock aiosmtpd if not installed in host Python environment
try:
    import aiosmtpd
except ImportError:
    mock_aiosmtpd = MagicMock()
    sys.modules["aiosmtpd"] = mock_aiosmtpd
    sys.modules["aiosmtpd.controller"] = mock_aiosmtpd.controller

import service as email_service


class TestEmailService(unittest.TestCase):

    def test_email_message_creation_and_send_reply(self):
        """
        Verify that send_reply constructs an EmailMessage object without NameError
        and sends it via smtplib.SMTP.
        """
        handler = email_service.EmailHandler()
        
        mock_smtp_instance = MagicMock()
        mock_smtp_class = MagicMock(return_value=mock_smtp_instance)
        mock_smtp_instance.__enter__.return_value = mock_smtp_instance

        with patch("smtplib.SMTP", mock_smtp_class):
            # Should execute cleanly without NameError: EmailMessage
            handler.send_reply(
                target_ip="127.0.0.1",
                target_email="user@test.local",
                original_subject="Test Subject",
                content="Test Email Body Content"
            )

        # Verify SMTP instantiation and message delivery
        mock_smtp_class.assert_called_once_with("127.0.0.1", email_service.REPLY_PORT)
        mock_smtp_instance.send_message.assert_called_once()
        
        sent_msg = mock_smtp_instance.send_message.call_args[0][0]
        self.assertEqual(sent_msg["Subject"], "Re: Test Subject")
        self.assertEqual(sent_msg["To"], "user@test.local")
        self.assertEqual(sent_msg.get_content().strip(), "Test Email Body Content")

    @patch.object(email_service.AgentRuntime, "process_email", return_value="Mocked AI Reply")
    def test_handle_data_end_to_end(self, mock_process_email):
        """
        Verify that handle_DATA extracts email contents and triggers send_reply.
        """
        handler = email_service.EmailHandler()
        
        mock_session = MagicMock()
        mock_session.peer = ("127.0.0.1", 12345)
        
        mock_envelope = MagicMock()
        mock_envelope.mail_from = "contestant@client-pod"
        mock_envelope.content = b"Subject: Calendar Query\n\nWhen is the meeting?"

        mock_smtp_instance = MagicMock()
        mock_smtp_instance.__enter__.return_value = mock_smtp_instance

        with patch("smtplib.SMTP", MagicMock(return_value=mock_smtp_instance)):
            import asyncio
            loop = asyncio.new_event_loop()
            asyncio.set_event_loop(loop)
            
            res = loop.run_until_complete(
                handler.handle_DATA(None, mock_session, mock_envelope)
            )
            loop.close()

        self.assertEqual(res, "250 OK")
        mock_process_email.assert_called_once()
        mock_smtp_instance.send_message.assert_called_once()


if __name__ == "__main__":
    unittest.main()
