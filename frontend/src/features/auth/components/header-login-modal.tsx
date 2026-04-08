import { Modal } from '@/components/ui/modal'
import { InlineLoginForm } from '@/features/auth/components/inline-login-form'

type HeaderLoginModalProps = {
  open: boolean
  onClose: () => void
  onLoggedIn: () => void
}

export function HeaderLoginModal({
  open,
  onClose,
  onLoggedIn,
}: HeaderLoginModalProps) {
  return (
    <Modal open={open} onClose={onClose} title="登录 ModelGate">
      <p className="mb-4 text-sm text-muted-foreground">
        使用注册时的用户名与密码登录。
      </p>
      <InlineLoginForm
        showAuxLinks
        onLoggedIn={() => {
          onClose()
          onLoggedIn()
        }}
      />
    </Modal>
  )
}
