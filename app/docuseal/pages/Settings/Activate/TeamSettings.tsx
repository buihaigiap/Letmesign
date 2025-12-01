import React, { useState, useEffect } from 'react';
import { 
  Typography, Box, Button, Table, TableBody, TableCell, TableContainer, 
  TableHead, TableRow, Paper, Dialog, DialogTitle, DialogContent, 
  DialogActions, TextField, Alert, CircularProgress, IconButton, Tooltip,
  Tab, Tabs
} from '@mui/material';
import CreateTemplateButton from '@/components/CreateTemplateButton';
import toast from 'react-hot-toast';
import { PenLine, Trash, Archive, ArchiveRestore, Users, Mail } from 'lucide-react';
import axios from 'axios';
import { useAuth } from '@/contexts/AuthContext';
import UpdatePro from '@/components/updatePro';
interface TeamMember {
  id: number;
  name: string;
  email: string;
  is_active: boolean;
  archived_at?: string | null;
  created_at: string;
}

const TeamSettings = () => {
  const { user: currentUser } = useAuth();
  const [open, setOpen] = useState(false);
  const [formData, setFormData] = useState({ name: '', email: '', password: '' });
  const [loading, setLoading] = useState(false);
  const [users, setUsers] = useState<TeamMember[]>([]);
  const [archivedUsers, setArchivedUsers] = useState<TeamMember[]>([]);
  const [fetchLoading, setFetchLoading] = useState(true);
  const [editingUser, setEditingUser] = useState<TeamMember | null>(null);
  const [currentTab, setCurrentTab] = useState(0);
  
  // console.log('user' , currentUser)
  const fetchTeamMembers = async () => {
    try {
      const token = localStorage.getItem('token');
      const response = await axios.get('/api/team/members', {
        headers: { Authorization: `Bearer ${token}` }
      });
      setUsers(response.data.users || []);
    } catch (err) {
      console.error('Failed to fetch team members:', err);
      toast.error('Failed to fetch team members');
    } finally {
      setFetchLoading(false);
    }
  };

  const fetchArchivedMembers = async () => {
    try {
      const token = localStorage.getItem('token');
      const response = await axios.get('/api/team/members/archived', {
        headers: { Authorization: `Bearer ${token}` }
      });
      setArchivedUsers(response.data.users || []);
    } catch (err) {
      console.error('Failed to fetch archived members:', err);
      toast.error('Failed to fetch archived members');
    }
  };

  useEffect(() => {
    fetchTeamMembers();
    fetchArchivedMembers();
  }, []);

  const handleClickOpen = () => {
    setEditingUser(null);
    setFormData({ name: '', email: '', password: '' });
    setOpen(true);
  };

  const handleClose = () => {
    setOpen(false);
    setEditingUser(null);
    setFormData({ name: '', email: '', password: '' });
  };

  const handleEdit = (user: TeamMember) => {
    if (currentUser?.id === user.id) {
      toast.error('You cannot edit your own account');
      return;
    }
    setEditingUser(user);
    setFormData({
      name: user.name,
      email: user.email,
      password: ''
    });
    setOpen(true);
  };

  const handleArchive = async (userId: number) => {
    if (currentUser?.id === userId) {
      toast.error('You cannot archive your own account');
      return;
    }
    if (!window.confirm('Are you sure you want to archive this member?')) return;
    try {
      const token = localStorage.getItem('token');
      await axios.post(`/api/team/members/${userId}/archive`, {}, {
        headers: { Authorization: `Bearer ${token}` }
      });
      toast.success('Member archived successfully');
      fetchTeamMembers();
      fetchArchivedMembers();
    } catch (err: any) {
      if (err.response?.status === 403) {
        toast.error('You cannot archive your own account');
      } else {
        toast.error(err.response?.data?.message || 'Failed to archive member');
      }
    }
  };

  const handleUnarchive = async (userId: number) => {
    if (currentUser?.id === userId) {
      toast.error('You cannot unarchive your own account');
      return;
    }
    try {
      const token = localStorage.getItem('token');
      await axios.post(`/api/team/members/${userId}/unarchive`, {}, {
        headers: { Authorization: `Bearer ${token}` }
      });
      toast.success('Member unarchived successfully');
      fetchTeamMembers();
      fetchArchivedMembers();
    } catch (err: any) {
      if (err.response?.status === 403) {
        toast.error('You cannot unarchive your own account');
      } else {
        toast.error(err.response?.data?.message || 'Failed to unarchive member');
      }
    }
  };

  const handleDelete = async (userId: number) => {
    if (currentUser?.id === userId) {
      toast.error('You cannot delete your own account');
      return;
    }
    if (!window.confirm('Are you sure you want to permanently delete this member?')) return;
    try {
      const token = localStorage.getItem('token');
      await axios.delete(`/api/team/members/${userId}`, {
        headers: { Authorization: `Bearer ${token}` }
      });
      toast.success('Member deleted successfully');
      fetchTeamMembers();
      fetchArchivedMembers();
    } catch (err: any) {
      if (err.response?.status === 403) {
        toast.error('You cannot delete your own account');
      } else {
        toast.error(err.response?.data?.message || 'Failed to delete member');
      }
    }
  };

  const handleSubmit = async () => {
    setLoading(true);
    try {
      const token = localStorage.getItem('token');
      
      if (editingUser) {
        // Prevent updating current user
        if (currentUser?.id === editingUser.id) {
          toast.error('You cannot edit your own account');
          setLoading(false);
          return;
        }
        
        // Update existing member
        const updateData: any = {
          name: formData.name,
          email: formData.email
        };
        
        await axios.put(`/api/team/members/${editingUser.id}`, updateData, {
          headers: { Authorization: `Bearer ${token}` }
        });
        toast.success('Member updated successfully');
      } else {
        // Create new member (all members are admin by default)
        const createData: any = {
          name: formData.name,
          email: formData.email,
          role: 'admin' // All members have admin access
        };
        
        if (formData.password) {
          createData.password = formData.password;
        }
        
        await axios.post('/api/team/members', createData, {
          headers: { Authorization: `Bearer ${token}` }
        });
        toast.success('Member added successfully');
      }
      
      fetchTeamMembers();
      handleClose();
    } catch (err: any) {
      toast.error(err.response?.data?.message || 'Operation failed');
    } finally {
      setLoading(false);
    }
  };

  const handleTabChange = (_event: React.SyntheticEvent, newValue: number) => {
    setCurrentTab(newValue);
  };

  const activeTableColumns = ['Name', 'Email', 'Created At', 'Actions'];
  const archivedTableColumns = ['Name', 'Email', 'Archived At', 'Actions'];
  if (currentUser?.subscription_status === 'free') {
    return (
      <>
        <UpdatePro />
      </>
    )
  }
  return (
    <Box>
      <Box display='flex' alignItems='center' justifyContent='space-between' mb={3}>
        <Box>
          <Typography variant="h5" sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <Users size={28} />
            Team Accounts
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
            Collaborate with your team - all members have full admin access
          </Typography>
        </Box>
        
        <CreateTemplateButton
          onClick={handleClickOpen}
          text="Add Member"
          background="linear-gradient(135deg, #4F46E5 0%, #7C3AED 100%)"
        />
      </Box>

      {/* Info Alert */}
      <Alert severity="info" sx={{ mb: 3, bgcolor: 'rgba(59, 130, 246, 0.1)', color: 'white' }}>
        All team members have full admin access and can collaborate on templates and submissions.
      </Alert>

    <Tabs 
        value={currentTab} 
        onChange={handleTabChange} 
        sx={{ 
            mb: 2,
            '& .MuiTab-root': { color: 'white !important' },     
            '& .Mui-selected': { color: 'white !important' },       
            '& .MuiTabs-indicator': { bgcolor: '#4F46E5' }
        }}
        >

        <Tab 
          label={
            <Box display="flex" alignItems="center" gap={1} sx={{ color: 'white' }}>
              <Users size={18} />
              Active Members ({users.length})
            </Box>
          }
          sx={{ color: 'white !important' }}
        />
        <Tab 
          label={
            <Box display="flex" alignItems="center" gap={1} sx={{ color: 'white' }}>
              <Archive size={18} />
              Archived ({archivedUsers.length})
            </Box>
          }
          sx={{ color: 'white !important' }}
        />
      </Tabs>

      {currentTab === 0 && (
        <TableContainer component={Paper} sx={{ bgcolor: 'rgba(15, 23, 42, 0.6)', backdropFilter: 'blur(10px)' }}>
          <Table sx={{ '& .MuiTableCell-root': { borderBottom: '1px solid rgba(255, 255, 255, 0.1)' } }}>
            <TableHead>
              <TableRow sx={{ bgcolor: 'rgba(79, 70, 229, 0.1)' }}>
                {activeTableColumns.map((column) => (
                  <TableCell key={column} sx={{ color: 'white', fontWeight: 'bold', py: 2 }}>
                    {column}
                  </TableCell>
                ))}
              </TableRow>
            </TableHead>
            <TableBody>
              {fetchLoading ? (
                <TableRow>
                  <TableCell colSpan={activeTableColumns.length} sx={{ color: 'white', textAlign: 'center', py: 4 }}>
                    <CircularProgress size={24} sx={{ color: '#4F46E5' }} />
                    <Typography sx={{ mt: 2 }}>Loading members...</Typography>
                  </TableCell>
                </TableRow>
              ) : users.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={activeTableColumns.length} sx={{ color: 'white', textAlign: 'center', py: 4 }}>
                    <Users size={48} style={{ opacity: 0.3, marginBottom: 16 }} />
                    <Typography>No active members yet. Click "Add Member" to invite team members.</Typography>
                  </TableCell>
                </TableRow>
              ) : (
                users.map((user) => (
                  <TableRow 
                    key={user.id}
                    sx={{ 
                      '&:hover': { bgcolor: 'rgba(79, 70, 229, 0.05)' },
                      transition: 'background-color 0.2s'
                    }}
                  >
                    <TableCell sx={{ color: 'white' }}>
                      <Box>
                        <Typography variant="body1" fontWeight="medium">{user.name}</Typography>
                      </Box>
                    </TableCell>
                    <TableCell sx={{ color: 'white' }}>
                      <Box display="flex" alignItems="center" gap={1}>
                        <Mail size={16} style={{ opacity: 0.5 }} />
                        {user.email}
                      </Box>
                    </TableCell>
                    <TableCell sx={{ color: 'rgba(255, 255, 255, 0.7)' }}>
                      {new Date(user.created_at).toLocaleDateString('en-US', { 
                        year: 'numeric', 
                        month: 'short', 
                        day: 'numeric' 
                      })}
                    </TableCell>
                    <TableCell>
                      {currentUser?.id === user.id ? (
                        <Typography variant="caption" sx={{ color: 'rgba(255, 255, 255, 0.5)', fontStyle: 'italic' }}>
                          (You)
                        </Typography>
                      ) : (
                        <Box display="flex" gap={0.5}>
                          <Tooltip title="Edit member">
                            <IconButton
                              size="small"
                              onClick={() => handleEdit(user)}
                              sx={{ 
                                color: '#60a5fa',
                                '&:hover': { bgcolor: 'rgba(96, 165, 250, 0.1)' }
                              }}
                            >
                              <PenLine size={16} />
                            </IconButton>
                          </Tooltip>
                          <Tooltip title="Archive member">
                            <IconButton
                              size="small"
                              onClick={() => handleArchive(user.id)}
                              sx={{ 
                                color: '#fb923c',
                                '&:hover': { bgcolor: 'rgba(251, 146, 60, 0.1)' }
                              }}
                            >
                              <Archive size={16} />
                            </IconButton>
                          </Tooltip>
                          <Tooltip title="Delete permanently">
                            <IconButton
                              size="small"
                              onClick={() => handleDelete(user.id)}
                              sx={{ 
                                color: '#f87171',
                                '&:hover': { bgcolor: 'rgba(248, 113, 113, 0.1)' }
                              }}
                            >
                              <Trash size={16} />
                            </IconButton>
                          </Tooltip>
                        </Box>
                      )}
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </TableContainer>
      )}

      {currentTab === 1 && (
        <TableContainer component={Paper} sx={{ bgcolor: 'rgba(15, 23, 42, 0.6)', backdropFilter: 'blur(10px)' }}>
          <Table sx={{ '& .MuiTableCell-root': { borderBottom: '1px solid rgba(255, 255, 255, 0.1)' } }}>
            <TableHead>
              <TableRow sx={{ bgcolor: 'rgba(251, 146, 60, 0.1)' }}>
                {archivedTableColumns.map((column) => (
                  <TableCell key={column} sx={{ color: 'white', fontWeight: 'bold', py: 2 }}>
                    {column}
                  </TableCell>
                ))}
              </TableRow>
            </TableHead>
            <TableBody>
              {archivedUsers.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={archivedTableColumns.length} sx={{ color: 'white', textAlign: 'center', py: 4 }}>
                    <Box display="flex" flexDirection="column" alignItems="center" justifyContent="center">
                      <Archive size={48} style={{ opacity: 0.3, marginBottom: 16 }} />
                      <Typography>No archived members</Typography>
                    </Box>
                  </TableCell>
                </TableRow>
              ) : (
                archivedUsers.map((user) => (
                  <TableRow 
                    key={user.id}
                    sx={{ 
                      opacity: 0.7,
                      '&:hover': { opacity: 1, bgcolor: 'rgba(251, 146, 60, 0.05)' },
                      transition: 'all 0.2s'
                    }}
                  >
                    <TableCell sx={{ color: 'white' }}>{user.name}</TableCell>
                    <TableCell sx={{ color: 'white' }}>{user.email}</TableCell>
                    <TableCell sx={{ color: 'rgba(255, 255, 255, 0.7)' }}>
                      {user.archived_at ? new Date(user.archived_at).toLocaleDateString('en-US', { 
                        year: 'numeric', 
                        month: 'short', 
                        day: 'numeric' 
                      }) : 'N/A'}
                    </TableCell>
                    <TableCell>
                      {currentUser?.id === user.id ? (
                        <Typography variant="caption" sx={{ color: 'rgba(255, 255, 255, 0.5)', fontStyle: 'italic' }}>
                          (You - Cannot modify)
                        </Typography>
                      ) : (
                        <Box display="flex" gap={0.5}>
                          <Tooltip title="Restore member">
                            <IconButton
                              size="small"
                              onClick={() => handleUnarchive(user.id)}
                              sx={{ 
                                color: '#34d399',
                                '&:hover': { bgcolor: 'rgba(52, 211, 153, 0.1)' }
                              }}
                            >
                              <ArchiveRestore size={16} />
                            </IconButton>
                          </Tooltip>
                          <Tooltip title="Delete permanently">
                            <IconButton
                              size="small"
                              onClick={() => handleDelete(user.id)}
                              sx={{ 
                                color: '#f87171',
                                '&:hover': { bgcolor: 'rgba(248, 113, 113, 0.1)' }
                              }}
                            >
                              <Trash size={16} />
                            </IconButton>
                          </Tooltip>
                        </Box>
                      )}
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </TableContainer>
      )}

      <Dialog 
        open={open} 
        onClose={handleClose} 
        maxWidth="sm" 
        fullWidth
        PaperProps={{
          sx: {
            bgcolor: '#1e293b',
            backgroundImage: 'linear-gradient(to bottom right, rgba(79, 70, 229, 0.05), rgba(124, 58, 237, 0.05))',
            backdropFilter: 'blur(10px)'
          }
        }}
      >
        <DialogTitle sx={{ color: 'white', borderBottom: '1px solid rgba(255, 255, 255, 0.1)' }}>
          <Box display="flex" alignItems="center" gap={1}>
            {editingUser ? <PenLine size={24} /> : <Users size={24} />}
            {editingUser ? 'Edit Team Member' : 'Add Team Member'}
          </Box>
        </DialogTitle>
        <DialogContent sx={{ mt: 2 }}>
          <TextField
            fullWidth
            label="Name"
            name="name"
            value={formData.name}
            onChange={(e) => setFormData({ ...formData, name: e.target.value })}
            margin="normal"
            required
            sx={{ 
              '& .MuiOutlinedInput-root': {
                color: 'white',
                '& fieldset': { borderColor: 'rgba(255, 255, 255, 0.2)' },
                '&:hover fieldset': { borderColor: 'rgba(79, 70, 229, 0.5)' },
                '&.Mui-focused fieldset': { borderColor: '#4F46E5' }
              },
              '& .MuiInputLabel-root': { color: 'rgba(255, 255, 255, 0.7)' }
            }}
          />
          <TextField
            fullWidth
            label="Email"
            name="email"
            type="email"
            value={formData.email}
            onChange={(e) => setFormData({ ...formData, email: e.target.value })}
            margin="normal"
            required
            sx={{ 
              '& .MuiOutlinedInput-root': {
                color: 'white',
                '& fieldset': { borderColor: 'rgba(255, 255, 255, 0.2)' },
                '&:hover fieldset': { borderColor: 'rgba(79, 70, 229, 0.5)' },
                '&.Mui-focused fieldset': { borderColor: '#4F46E5' }
              },
              '& .MuiInputLabel-root': { color: 'rgba(255, 255, 255, 0.7)' }
            }}
          />
          {!editingUser && (
            <TextField
              fullWidth
              label="Password (optional - auto-generated if empty)"
              name="password"
              type="password"
              value={formData.password}
              onChange={(e) => setFormData({ ...formData, password: e.target.value })}
              margin="normal"
              helperText="Leave empty to auto-generate a secure password"
              sx={{ 
                '& .MuiOutlinedInput-root': {
                  color: 'white',
                  '& fieldset': { borderColor: 'rgba(255, 255, 255, 0.2)' },
                  '&:hover fieldset': { borderColor: 'rgba(79, 70, 229, 0.5)' },
                  '&.Mui-focused fieldset': { borderColor: '#4F46E5' }
                },
                '& .MuiInputLabel-root': { color: 'rgba(255, 255, 255, 0.7)' },
                '& .MuiFormHelperText-root': { color: 'rgba(255, 255, 255, 0.5)' }
              }}
            />
          )}
          
          {!editingUser && (
            <Alert severity="info" sx={{ mt: 2, bgcolor: 'rgba(59, 130, 246, 0.1)' , color:'white' }}>
              Member will have full admin access and receive an email invitation.
            </Alert>
          )}
        </DialogContent>
        <DialogActions sx={{ p: 2.5, borderTop: '1px solid rgba(255, 255, 255, 0.1)' }}>
          <Button 
            onClick={handleClose} 
            sx={{ color: 'rgba(255, 255, 255, 0.7)' }}
            disabled={loading}
          >
            Cancel
          </Button>
          <Button 
            onClick={handleSubmit} 
            variant="contained" 
            disabled={loading || !formData.name || !formData.email}
            sx={{
              background: 'linear-gradient(135deg, #4F46E5 0%, #7C3AED 100%)',
              '&:hover': {
                background: 'linear-gradient(135deg, #4338CA 0%, #6D28D9 100%)'
              }
            }}
          >
            {loading ? <CircularProgress size={24} /> : editingUser ? 'Update Member' : 'Add Member'}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
};

export default TeamSettings;
