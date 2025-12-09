import { useEffect, useState, useRef } from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
import upstashService from '../../ConfigApi/upstashService';
import toast from 'react-hot-toast';
import TemplatesGrid from './TemplatesGrid';
import FoldersList from '../../components/FoldersList';
import { Button, TextField, IconButton } from '@mui/material';
import { Box } from '@mui/material';
import { Pencil, ChevronLeft, Trash2 } from 'lucide-react';
import NewTemplateModal from '../../components/NewTemplateModal';
import CreateTemplateButton from '../../components/CreateTemplateButton';

interface Folder {
  id: number;
  name: string;
  parent_folder_id?: number;
  children?: Folder[];
}

interface Template {
  id: number;
  name: string;
  file_url: string;
  created_at: string;
  user_id: number;
  slug: string;
  updated_at: string;
}

const FolderPage: React.FC = () => {
  const { folderId } = useParams<{ folderId: string }>();
  const [folders, setFolders] = useState<Folder[]>([]);
  const [templates, setTemplates] = useState<Template[]>([]);
  const [loading, setLoading] = useState(true);
  const [currentFolder, setCurrentFolder] = useState<Folder | null>(null);
  const [parentFolder, setParentFolder] = useState<Folder | null>(null);
  const [isEditing, setIsEditing] = useState(false);
  const [newName, setNewName] = useState('');
  const [showNewTemplateModal, setShowNewTemplateModal] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const navigate = useNavigate();
  
  const findFolderById = (folders: Folder[], id: number): Folder | null => {
    for (const folder of folders) {
      if (folder.id === id) {
        return folder;
      }
      if (folder.children) {
        const found = findFolderById(folder.children, id);
        if (found) return found;
      }
    }
    return null;
  };

  const handleEditStart = () => {
    setIsEditing(true);
    setNewName(currentFolder?.name || '');
  };

  const handleEditSave = async () => {
    if (!newName.trim() || !currentFolder) return;
    try {
      const response = await upstashService.updateFolder(currentFolder.id, { name: newName.trim() });
      if (response.success) {
        setIsEditing(false);
        // Update local state
          setCurrentFolder({ ...currentFolder, name: newName.trim() });
          toast.success('Folder name updated successfully');
      } else {
        toast.error('Error updating folder name');
      }
    } catch (error) {
      toast.error('Error updating folder name');
    }
  };

  const handleEditCancel = () => {
    setIsEditing(false);
    setNewName('');
  };

  const handleDeleteFolder = async () => {
    if (!currentFolder) return;
    const confirmDelete = window.confirm(
      `Are you sure you want to delete the folder "${currentFolder.name}"? This action cannot be undone and will also delete all templates and subfolders inside it.`
    );

    if (!confirmDelete) return;

    try {
      const response = await upstashService.deleteFolder(currentFolder.id);
      if (response.success) {
        // Navigate back to parent folder or home
        if (parentFolder) {
          navigate(`/folders/${parentFolder.id}`);
        } else {
          navigate('/');
        }
        toast.success('Folder deleted successfully');
      } 
    } catch (error) {
      toast.error(error?.error);
    }
  };
  const fetchData = async () => {
    try {
      setLoading(true);
      const allFolders = await upstashService.getFolders();
      const folderTemplates = await upstashService.getFolderTemplates(Number(folderId));
      // Find the current folder in the tree
      const currentFolder = findFolderById(allFolders.data, Number(folderId));
      // Get subfolders: the children of the current folder
      const subFolders = currentFolder?.children || [];
      setFolders(subFolders);
      setCurrentFolder(currentFolder);
      // Find parent folder
      if (currentFolder?.parent_folder_id) {
        const parent = findFolderById(allFolders.data, currentFolder.parent_folder_id);
        setParentFolder(parent);
      } else {
        setParentFolder(null); // root, back to home
      }
      setTemplates(folderTemplates.data);
    } catch (error) {
      toast.error('Error fetching folder data');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (folderId) {
      fetchData();
    }
  }, [folderId]);

  useEffect(() => {
    if (inputRef.current && isEditing) {
      inputRef.current.style.width = 'auto';
      inputRef.current.style.width = inputRef.current.scrollWidth + 'px';
    }
  }, [newName, isEditing]);

  return (
<Box >
      {/* Back Button */}
      <Box marginTop={2}>
        <Button
          component={Link}
          to={parentFolder ? `/folders/${parentFolder.id}` : '/'}
          startIcon={<ChevronLeft size={16} />}
          sx={{
            color: 'white',
            textTransform: 'none',
            fontSize: '1rem',   
        }}
        >
          {parentFolder ? parentFolder.name : 'Home'}
        </Button>
      </Box>

      {currentFolder && (
        <div style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: '2rem',
          padding: '1rem',
          borderRadius: '8px'
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' , maxWidth:"85%"}}>
            {isEditing ? (
              <>
                <TextField
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  size="small"
                  inputRef={inputRef}
                  sx={{width: 'auto', input: { color: 'white' } }}
                />
                <Button onClick={handleEditSave} variant="contained">Save</Button>
                <Button onClick={handleEditCancel}  sx={{ color: 'white' }}>Cancel</Button>
              </>
            ) : (
              <>
                <h1 style={{ color: 'white', margin: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', flex: 1 }}>{currentFolder.name}</h1>
                <Pencil onClick={handleEditStart} size={20} className='cursor-pointer'/>
                <Trash2 onClick={handleDeleteFolder} size={20} className='cursor-pointer text-red-400 hover:text-red-300'/>
              </>
            )}
          </div>
          <div style={{ display: 'flex', gap: '1rem' }}>
            <CreateTemplateButton width='200px' onClick={() => setShowNewTemplateModal(true)} text="Create New Template" />
          </div>
        </div>
      )}
        <FoldersList folders={folders} title="" />
        {templates.length > 0 && (
            <div>
                <TemplatesGrid 
                    templates={templates} 
                    onRefresh={fetchData} 
                    currentFolderId={Number(folderId)} />
            </div>
        )}
        <NewTemplateModal
          open={showNewTemplateModal}
          onClose={() => setShowNewTemplateModal(false)}
          folderId={Number(folderId)}
          onSuccess={fetchData}
        />
    </Box>
  );
};

export default FolderPage;